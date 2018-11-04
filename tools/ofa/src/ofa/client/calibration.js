/**
 * Created by Darshana 27.11.2016
 * Calibration tool kit, seperated from calibration editor for cleanliness :P
 *
 * Recieves 3D points and corresponding 2D points. Using NAO's projection
 * Things are calibrated.
 *
 * 20.07.2017 Removed explicit cv-XYZ coord system.
 */
var math = require('mathjs');
var Solvers = require('./libs/math-tools').Solvers;
var fminsearch = require('./libs/fminsearch');
var _ = require('underscore');

function CalibrationToolkit(args) {
  this.init(args);
}

// OpenCV-XYZ frame to robot's XYZ frame (roll pitch yaw) conversions
CalibrationToolkit.xyz2rpy =
    math.matrix([[0, 0, 1, 0], [-1, 0, 0, 0], [0, -1, 0, 0], [0, 0, 0, 1]]);
CalibrationToolkit.rpy2xyz = math.inv(CalibrationToolkit.xyz2rpy);

CalibrationToolkit.camera2HeadUncalibRot = {
  "top": 0.0209,
  "bottom": 0.6929
};
CalibrationToolkit.camera2HeadUncalibPos = {
  "top": [58.71, 0, 63.64],
  "bottom": [50.71, 0, 17.74]
};

// similar to array.prototype.map, except this returns an object :P
CalibrationToolkit.arrToObj = function(arr, fcn) {
  let tempObj = {};
  for (let val of arr) {
    tempObj[val] = (fcn) ? fcn(val) : {};
  }
  return tempObj;
};

/**
 * constructor
 */
CalibrationToolkit.prototype.init = function(args) {

  if (!args) {
    args = new Object();
  }

  this.cameraNames =
      (args.cameraName != null) ? args.cameraNames : ["top", "bottom"];

  // this.canvas = (args.canvas != null) ? args.canvas :
  // document.getElementById("canvas");
  // this.ctx = (args.context != null) ? args.context :
  // this.canvas.getContext("2d");
  this.canvasArr = (args.canvasArr != null) ?
      args.canvasArr :
      CalibrationToolkit.arrToObj(
          this.cameraNames,
          (name) => {return document.getElementById("canvas")});
  this.ctxArr = (args.ctxArr != null) ?
      args.ctxArr :
      CalibrationToolkit.arrToObj(
          this.cameraNames,
          (name) => {return this.canvasArr[name].getContext("2d")});
  if (args.arucoBoard3D) {
    this.arucoBoard3D = args.arucoBoard3D;
  } else {  // default aruco marker set, 3D coordinates relative to robot's
            // ground point.
    alert("No Calibration pattern give. Defaults used");
    this.arucoBoard3D = {};
  }

  /**
   * Default projection config
   * copied from nao repo
   */
  if (!args.projectionConfig) {
    this.projectionConfig = {
      "top_ext":
          [0.01968503937007871, 0.06692913385826771, 0.04330708661417326],
      "top_fc": [0.8956836983962191, 1.5894125993658377],
      "top_cc": [0.5002333653968702, 0.4963466501955577],
      "bottom_ext":
          [0.003937007874015741, 0.04330708661417326, 0.05905511811023623],
      "bottom_fc": [0.973359375, 1.3021874999999998],
      "bottom_cc": [0.50703125, 0.5194791666666666],
      "torsoCalibration": [0, 0]
    };
  } else {
    this.projectionConfig = args.projectionConfig;
  }
  if (!this.projectionConfig.torsoCalibration) {
    this.projectionConfig["torsoCalibration"] = [0, 0];
  }
  this.camera2Head = new Object();

  // These values are from
  // http://doc.aldebaran.com/2-1/family/robots/video_robot.html
  // They specify the translation and rotation of the cameras to the HEAD_PITCH
  // joint.
  this.camera2Head_uncalib =
      CalibrationToolkit.arrToObj(this.cameraNames, (name) => {
        return CalibrationToolkit.getFullTransform(
            CalibrationToolkit.getRotMat(
                0, CalibrationToolkit.camera2HeadUncalibRot[name], 0),
            CalibrationToolkit.camera2HeadUncalibPos[name]);
      });

  // camera intrinsic
  this.cam_cc = CalibrationToolkit.arrToObj(
      this.cameraNames, (name) => {return this.projectionConfig[name + "_cc"]});
  this.cam_fc = CalibrationToolkit.arrToObj(
      this.cameraNames, (name) => {return this.projectionConfig[name + "_fc"]});
  this.cam_ext = CalibrationToolkit.arrToObj(
      this.cameraNames,
      (name) => {return this.projectionConfig[name + "_ext"]});

  // aruco marker size, default 80mm
  if (!this.arucoBoard3D.markerSize) {
    console.log("WARNING : Marker size not specified");
  }
  this.markerSize =
      (this.arucoBoard3D.markerSize) ? this.arucoBoard3D.markerSize : 50;
};

/**
 * Use this for multi-image-extrinsic calibration
 *
 * snapshots:{top:[
 *      {
 *          threeDeePoints:[],
 *          twoDeePoints:[],
 *          head2Torso:{},
 *          torso2Ground:{},
 *          camera2Ground_inv:[]
 *      }
 * ]
 */

CalibrationToolkit.prototype.extrinsicMultiImage = function(params, callback) {
  if (callback) {
    this.calibCallback = callback;
  } else {
    this.calibCallback = function() {};
  }
  if (!params) {
    this.callback();
    return false;
  }
  params.doTorso = params.doTorso || false;
  // for (let name of this.cameraNames) {
  //   if (params.snapshots[name]) {
  //     for (let i = 0; i < params.snapshots[name].length; i++) {
  //       params.snapshots[name][i].camera`2Ground_invXYZ = math.multiply(
  //           CalibrationToolkit.rpy2xyz,
  //           params.snapshots[name][i].camera2Ground_inv);
  //     }
  //   }
  // }
  this.calculateMatrices(
      params.snapshots, params.doExtrinsic, params.doIntrinsic, params.doTorso);
};

/**
 * Function responsible for all calibrations, etc
 *
 * Mod: 2017.03.26 support multiple images
 *
 * NOTE: This iterative process can be done with existing calibration system -
 *      if the goal points are pointed out by mouse clicks.
 * @param snapshots aruco marker object array.
 */

CalibrationToolkit.prototype.calculateMatrices = function(
    snapshots, doExtrinsic = undefined, doIntrinsic = undefined,
    doTorso = false) {
  // This object will be sent back via the calibration callback
  doExtrinsic = (doExtrinsic) ?  // if array is passed, selective calib.
      doExtrinsic :
      CalibrationToolkit.arrToObj(this.cameraNames, (name) => {doExtrinsic});

  doIntrinsic = (doIntrinsic) ?
      doIntrinsic :
      CalibrationToolkit.arrToObj(this.cameraNames, (name) => {doIntrinsic});

  let calibCallbackData =
      CalibrationToolkit.arrToObj(this.cameraNames, (name) => {
        return {
          cameraName: name,
          cam_cc: new Array(),
          cam_fc: new Array(),
          cam_ext: new Array(),
          doExtrinsic: doExtrinsic[name],
          doIntrinsic: doIntrinsic[name]
        }
      });
  calibCallbackData.doTorso = doTorso;

  if (snapshots["top"].length <= 0 &&
      snapshots["bottom"].length <= 0) {  // Check this first!!
    console.log("Empty marker list passed, aborting");
    this.calibCallback(calibCallbackData);
    return;
  }

  // calculate pose for each aruco marker and then project the 3D points via
  // camera2Ground_inv * cam2px and compare.
  // TODO modify to digest charuco board
  var markerSizeHalf = this.markerSize / 2;
  var canvasWidth = CalibrationToolkit.arrToObj(
      this.cameraNames, (name) => {return this.canvasArr[name].width});
  var canvasHeight = CalibrationToolkit.arrToObj(
      this.cameraNames, (name) => {return this.canvasArr[name].height});
  var scaled_fc = CalibrationToolkit.arrToObj(
      this.cameraNames, (name) => {return new Array()});
  var scaled_cc = CalibrationToolkit.arrToObj(
      this.cameraNames, (name) => {return new Array()});

  // Nao stores focal point and image centers in relative values; need to
  // multiply to get absolutes for a given image
  for (let name of this.cameraNames) {
    scaled_cc[name][0] = this.cam_cc[name][0] * canvasWidth[name];
    scaled_cc[name][1] = this.cam_cc[name][1] * canvasHeight[name];
    scaled_fc[name][0] = this.cam_fc[name][0] * canvasWidth[name];
    scaled_fc[name][1] = this.cam_fc[name][1] * canvasHeight[name];
  }

  let cameraMatrix = CalibrationToolkit.arrToObj(this.cameraNames, (name) => {
    return CalibrationToolkit.getCameraMatrix(scaled_fc[name], scaled_cc[name]);
  });

  // 4 corners in respect to each aruco marker's 3D position
  var threeDeeWRTArucoMarker = [
    [-markerSizeHalf, markerSizeHalf, markerSizeHalf, -markerSizeHalf],
    [markerSizeHalf, markerSizeHalf, -markerSizeHalf, -markerSizeHalf],
    [0, 0, 0, 0], [1, 1, 1, 1]
  ];

  var combinedMarkerPoints =
      CalibrationToolkit.arrToObj(this.cameraNames, () => {return new Array()});
  /**
   * Iterate through marker list, make the transformation matrices.
   * NOTE: These use OpenCV style x-y-z coordinate frame.
   * Conversion can be done with CalibrationToolkit.xyz2rpy
   */
  for (let name of this.cameraNames) {
    if (!snapshots[name] && snapshots[name].length <= 0) {
      continue;
    }

    for (let snapIndex = 0; snapIndex < snapshots[name].length; snapIndex++) {
      let snapshot = snapshots[name][snapIndex];
      if (snapshot.twoDeePoints) {
        for (let twoDeePoint of snapshot.twoDeePoints) {
          combinedMarkerPoints[name].push(twoDeePoint);
        }
      } else {
        console.error("No marker Points found!!!");
      }
    }
  }
  /**
   * Call this function for intrinsic processing.
   */
  var intrinsicFunc = function(callback) {
    //     if(markers.length >0){// Check this first!!
    for (let name of this.cameraNames) {
      if (!doIntrinsic[name]) {
        // callback();
        // return;
        continue;
      }
      /**
       * parameters to be optimized by iterative solver
       */
      let camMatrixParams = [
        scaled_fc[name][0], scaled_fc[name][1], scaled_cc[name][0],
        scaled_cc[name][1]
      ];  // parameter array sent to solver

      // the function that is called by solver get this object as constants/
      // extra
      // stuff
      let iteratorArgs = {
        snapshots: snapshots[name],
        threeDee: threeDeeWRTArucoMarker
      };

      // check if calibration is good TODO make this optional in future
      let newOuts = this.iterateForIntrinsics(iteratorArgs, camMatrixParams);
      let totalError = 0;

      if (newOuts.length != combinedMarkerPoints[name].length) {
        console.log(
            "CRITICAL :Array length mismatch", newOuts.length,
            combinedMarkerPoints[name].length);
      }

      for (let i = 0; i < newOuts.length; i++) {
        totalError += Math.pow((combinedMarkerPoints[name][i] - newOuts[i]), 2);
      }
      //             console.log(totalError, newOuts.length);
      if (isNaN(totalError)) {
        console.log("Critical Failure, NaN for Sum of Error Squared");
        alert("CRITICAL : NaN for Sum of Error Squared");
      }

      let sumOfErrSq = totalError / newOuts.length;
      let continueIntrinsicCalib = false;

      if (sumOfErrSq > 6) {  // if avg error is less than 0.2 px.
        console.log("Intrinsic params are bad " + sumOfErrSq);
        continueIntrinsicCalib = true;
      } else {
        console.log("Intrinsic params okay, skipping calib :" + sumOfErrSq);
        continueIntrinsicCalib = false;
      }

      if (continueIntrinsicCalib) {
        newOuts = [];
        totalError = 0;
        sumOfErrSq = 0;

        console.log("Intrinsic Iteration start", new Date());
        // Iterative search to find best matrix
        let refinedParams = fminsearch(
            this.iterateForIntrinsics, camMatrixParams, iteratorArgs,
            combinedMarkerPoints[name], {maxIter: 400});

        // check state of convergence
        newOuts = this.iterateForIntrinsics(iteratorArgs, refinedParams);

        if (newOuts.length != combinedMarkerPoints[name].length) {
          console.log(
              "Array l;ength mismatch", newOuts.length,
              combinedMarkerPoints[name].length);
        }

        for (let i = 0; i < newOuts.length; i++) {
          totalError +=
              Math.pow((combinedMarkerPoints[name][i] - newOuts[i]), 2);
          if (i % 2 == 0) {
            this.ctxArr[name].strokeStyle = "purple";
            this.ctxArr[name].strokeRect(
                newOuts[i] - 2, newOuts[i + 1] - 2, 4, 4);
            this.ctxArr[name].strokeStyle = "green";
            this.ctxArr[name].strokeRect(
                combinedMarkerPoints[name][i] - 2,
                combinedMarkerPoints[name][i + 1] - 2, 4, 4);
          }
        }
        sumOfErrSq = totalError / newOuts.length;

        if (sumOfErrSq <= 16) {  // if avg error is less than 0.2 px.
          console.log(
              "Intrinsic Iteration Success, avg. error in px. " + sumOfErrSq);
        } else {
          console.log("Intrinsic Iteration Failed", sumOfErrSq);
        }

        // update camera matrix
        this.cam_fc[name][0] = refinedParams[0] / canvasWidth[name];
        this.cam_fc[name][1] = refinedParams[1] / canvasHeight[name];
        this.cam_cc[name][0] = refinedParams[2] / canvasWidth[name];
        this.cam_cc[name][1] = refinedParams[3] / canvasHeight[name];

        // update callback data pack.
        calibCallbackData[name].cam_fc = _.clone(this.cam_fc[name]);
        calibCallbackData[name].cam_cc = _.clone(this.cam_cc[name]);

        scaled_fc[name][0] = refinedParams[0];
        scaled_fc[name][1] = refinedParams[1];
        scaled_cc[name][0] = refinedParams[2];
        scaled_cc[name][1] = refinedParams[3];

        console.log(
            name + " scaled_fc", scaled_fc[name], "scaled_cc", scaled_cc[name]);
        console.log(
            name + " cam_fc", this.cam_fc[name], "cam_cc", this.cam_cc[name]);

        cameraMatrix[name] = CalibrationToolkit.getCameraMatrix(
            scaled_fc[name], scaled_cc[name]);
        this.drawOriginAxis(
            cameraMatrix[name],
            snapshots[name][snapshots[name].length - 1].transforms, name);
      }  // endif continueIntrinsicCalib
    }
    callback();

  }.bind(this);

  /**
   * Extrinsic function
   *
   * Start extrinsic Calib
   * NOTE: All transformations performed in OpenCV XYZ coordinate format.
   * Nao_x = z, nao_y = -x, nao_z = -y
   */
  var extrinsicFunc = function(callback) {
    if (!(doExtrinsic["top"] || doExtrinsic["bottom"])) {
      callback();
      return;
    }

    var arucoThreeDeeSet = [new Array(), new Array(), new Array(), new Array()];
    let threeDeePointCount = 0;
    for (let name of this.cameraNames) {
      if (!doExtrinsic[name]) {
        continue;
      }
      for (let snapIndex = 0; snapIndex < snapshots[name].length; snapIndex++) {
        let snapshot = snapshots[name][snapIndex];
        threeDeePointCount += snapshot.threeDeePointCount;
        for (let i = 0; i < snapshot.threeDee.length; i++) {
          arucoThreeDeeSet[i].push.apply(
              arucoThreeDeeSet[i], snapshot.threeDee[i]);
        }
      }
      CalibrationToolkit.projectMarkerOutline(
          cameraMatrix[name],
          snapshots[name][snapshots[name].length - 1].camera2Ground_inv,
          snapshots[name][snapshots[name].length - 1].threeDee,
          this.ctxArr[name], "green", 0);
    }
    /**
     * Step 2. Iteratively solve for better extrinsic angles.
     */

    // additional work needed for levenburg, fix this in future.

    arucoThreeDeeSet = math.matrix(arucoThreeDeeSet);
    let arucoThreeDeeArr =
        math.flatten(math.transpose(arucoThreeDeeSet)).toArray();

    let isOneCamera = !(doExtrinsic["top"] && doExtrinsic["bottom"]) &&
        (doExtrinsic["top"] || doExtrinsic["bottom"]);
    // Prep for iterative solver
    var extrinsicParams = [];
    if (isOneCamera) {
      var cameraName = (doExtrinsic["top"]) ? "top" : "bottom";
      extrinsicParams.concat(this.cam_ext[cameraName]);
      extrinsicParams.concat([
        this.projectionConfig.torsoCalibration[0],
        this.projectionConfig.torsoCalibration[1], 0
      ]);
    } else {
      extrinsicParams = [
        this.cam_ext["top"][0], this.cam_ext["top"][1], this.cam_ext["top"][2],
        this.cam_ext["bottom"][0], this.cam_ext["bottom"][1],
        this.cam_ext["bottom"][2], this.projectionConfig.torsoCalibration[0],
        this.projectionConfig.torsoCalibration[1],
        0
      ];  // last two for torso calib
    }
    var extrinsicIteratorArgs = {
      snapshots: snapshots,
      cameraMatrix: cameraMatrix,
      camera2Head_uncalib: this.camera2Head_uncalib,
      torsoCalib: doTorso,
      cameraNames: this.cameraNames,
      doExtrinsic: doExtrinsic
    };

    console.log(
        "3D point count == combinedMarkerPoints.len? " +
        (threeDeePointCount == (combinedMarkerPoints["top"].length +
                                combinedMarkerPoints["bottom"].length)));
    console.log(
        "f(x).length = y.length ? " +
        (this.iterateForExtrinsics(
                 arucoThreeDeeArr, extrinsicParams, extrinsicIteratorArgs)
             .toArray()
             .length == (combinedMarkerPoints["top"].length +
                         combinedMarkerPoints["bottom"].length)));
    console.log("levenbergMarquardtFnc start extinrinsics", new Date());

    var extrinsicOptimResult;

    /**
     * Callback after optimization is complete.
     * Since this is a nested function, it has access to the variables outside
     * its scope and within parent's scope.
     */
    let onExtrinsicOptimComplete = function() {
      let refinedParams = extrinsicOptimResult.solution;
      var newOuts = new Array();
      // run the projection manually, used to check convergence
      newOuts =
          this.iterateForExtrinsics({}, refinedParams, extrinsicIteratorArgs)
              .toArray();

      // Cannot happen, if happens, something extremely buggy!
      if (newOuts.length != (combinedMarkerPoints["top"].length +
                             combinedMarkerPoints["bottom"].length)) {
        console.log(
            "CRITICAL : Array length mismatch extinrinsics", newOuts.length,
            (combinedMarkerPoints["top"].length +
             combinedMarkerPoints["bottom"].length));
      }

      // Print the values!
      calibCallbackData.torsoCalibration = _.clone((isOneCamera)?refinedParams.slice(4, 6):refinedParams.slice(6, 8));
      calibCallbackData["top"].cam_ext = _.clone(refinedParams.slice(0, 3));
      calibCallbackData["bottom"].cam_ext = _.clone(refinedParams.slice(3, 6));
      console.log(
          "Torso Calib Angles for (Rad)",
          JSON.stringify(calibCallbackData.torsoCalibration, null, 4));
      console.log("Delta X", refinedParams[5]);

      var totalError = 0;
      for (let name of this.cameraNames) {
        if (!doExtrinsic[name]) {
          continue;
        }
        // Get sum of error squared and draw yellow squares around aruco
        // corners.
        for (var i = 0; i < newOuts.length; i++) {
          totalError +=
              Math.pow((combinedMarkerPoints[name][i] - newOuts[i]), 2);
          // if (i % 2 == 0) {
          //   this.ctxArr[name].strokeStyle = "yellow";
          //   this.ctxArr[name].strokeRect(
          //       newOuts[i] - 2, newOuts[i + 1] - 2, 4, 4);
          // }
        }
        console.log(
            "Extrinsic Angles for (Rad)" + name + " camera ",
            JSON.stringify(calibCallbackData[name].cam_ext, null, 4));
      }

      if (isNaN(extrinsicOptimResult.finalCost)) {
        console.log("Critical Failure, NaN for Sum of Error Squared");
        alert("CRITICAL : NaN for Sum of Error Squared");
      }

      if (extrinsicOptimResult.finalCost <=
          5) {  // if avg error is less than 0.2 px.
        console.log(
            "Extrinsic Iteration Success, avg. error in px. " +
            extrinsicOptimResult.finalCost);
      } else {
        console.log(
            "Extrinsic Iteration: too much error",
            extrinsicOptimResult.finalCost);
      }

      /**
       * Multi image version;
       */
      for (let name of this.cameraNames) {
        if (!doExtrinsic[name]) {
          continue;
        }
        for (let snapIndex = 0; snapIndex < snapshots[name].length;
             snapIndex++) {
          // var torsoRot = CalibrationToolkit.getRotMat(refinedParams[4],
          // refinedParams[3], 0);
          var torsoRot = math.multiply(
              CalibrationToolkit.getRotY(refinedParams[6]),
              CalibrationToolkit.getRotX(refinedParams[7]));
          var torsoCalibration = CalibrationToolkit.getFullTransform(torsoRot);

          let snapshot = snapshots[name][snapIndex];  // Du
          // build camera2Ground matrix based on the new values
          var rotM = {};
          if (name == "top") {
            rotM = CalibrationToolkit.getRotMat(
                refinedParams[0], refinedParams[1], refinedParams[2]);
          } else {
            rotM = CalibrationToolkit.getRotMat(
                refinedParams[3], refinedParams[4], refinedParams[5]);
          }

          var camera2Head = math.multiply(
              this.camera2Head_uncalib[name],
              CalibrationToolkit.getFullTransform(rotM));

          var camera2Ground = math.multiply(
              snapshot.torso2Ground,
              math.multiply(torsoCalibration, snapshot.head2Torso));
          camera2Ground = math.multiply(camera2Ground, camera2Head);

          snapshot.newCamera2Ground_invXYZ = math.multiply(
              CalibrationToolkit.rpy2xyz, math.inv(camera2Ground));
          snapshot.newCamera2Ground_inv = math.inv(camera2Ground);
        }
        /**
 * Draw the transformation using new values MOVE TO UI
 */
        CalibrationToolkit.projectMarkerOutline(
            cameraMatrix[name],
            snapshots[name][snapshots[name].length - 1].newCamera2Ground_inv,
            snapshots[name][snapshots[name].length - 1].threeDee,
            this.ctxArr[name], "yellow", refinedParams[8]);
      }
    }.bind(this);

    Solvers
        .levenbergMarquardtFnc(
            arucoThreeDeeArr,
            (doExtrinsic["top"] ? combinedMarkerPoints["top"] : [])
                .concat(
                    (doExtrinsic["bottom"] ? combinedMarkerPoints["bottom"] :
                                             [])),
            this.iterateForExtrinsics, extrinsicParams, extrinsicIteratorArgs,
            {maxIter: 80, convergeTol: 0.2})
        .then((output) => {
          console.log("levenbergMarquardtFnc end extinrinsics", new Date());
          extrinsicOptimResult = output;
          onExtrinsicOptimComplete();
          callback();
        });

  }.bind(this);

  // Calling everything in a async way. This helps to put heavy processing in
  // the queue and later fully go for web workers
  intrinsicFunc(function() {
    if (doExtrinsic) {
      extrinsicFunc(function() {
        // call the callback :P
        let output = {};
        for (let name of this.cameraNames) {
          if (doIntrinsic[name] || doExtrinsic[name]) {
            output[name] = calibCallbackData[name];
          }
        }
        if (doTorso) {
          output.doTorso = calibCallbackData.doTorso;
          output.torsoCalibration = calibCallbackData.torsoCalibration;
        }
        this.calibCallback(output);
      }.bind(this));
    } else {  // endif doExtrinsic
      let output = {};
      for (let name of this.cameraNames) {
        if (doIntrinsic[name]) {
          output[name] = calibCallbackData[name];
        }
      }
      this.calibCallback(output);
    }
  }.bind(this));
};

/**
 * Iterated function by solver - Get 3D points given in args, then use the
 * transform and return 2D points
 * @param args
 * @param solveForParams array containing the parameters to solve, here we send
 * fc.x, fc.y, cc.x, cc.y
 */

CalibrationToolkit.prototype.iterateForIntrinsics = function(
    args, solveForParams) {
  // var transforms = args.transforms;
  var threeDee = args.threeDee;
  var twoDeeArr = new Array();
  var cameraMatrix = CalibrationToolkit.getCameraMatrix(
      solveForParams.slice(0, 2), solveForParams.slice(2, 4));
  for (const snapshot of args.snapshots) {
    for (const transform of snapshot.transforms) {
      var twoDeeFinal = CalibrationToolkit.transform3Dto2DRPY(
          cameraMatrix, transform, threeDee);
      var twoDeeFinalSize = math.size(twoDeeFinal).toArray();
      for (var i = 0; i < twoDeeFinalSize[1]; i++) {
        twoDeeArr.push(twoDeeFinal.subset(math.index(0, i)));
        twoDeeArr.push(twoDeeFinal.subset(math.index(1, i)));
      }
    }
  }
  return twoDeeArr;
};

/**
 * Iterated function by solver - Get 3D points given in args, then use the
 * transform and return 2D points
 *
 * snapshots:[
 *      {
 *          markerPoints:[], OR give markers. In future, give 2D points and
 * corresponding 3D points only.
 *          head2Torso:{},
 *          torso2Ground:{},
 *          camera2Ground_inv:{}
 *      }
 * ]
 * @param args
 * @param solveForParams array containing the parameters to solve, here we send
 * fc.x, fc.y, cc.x, cc.y
 */
CalibrationToolkit.prototype.iterateForExtrinsics = function(
    input, solveForParams, args) {

  var twoDeeArr = new Array();
  var cameraMatrix = args.cameraMatrix;
  const isOneCamera = !(args.doExtrinsic["top"] && args.doExtrinsic["bottom"]);
  /**
   * Some matrices change with each snapshot/ posture of nao.
   */
  // console.log(args.snapshots);
  for (let name of args.cameraNames) {
    if (!args.doExtrinsic[name]) {
      continue;
    }
    for (const snapshot of args.snapshots[name]) {
      var cameraExtrinsics = [];
      if (name == "top" || isOneCamera) {
        cameraExtrinsics = solveForParams.slice(0, 3);
      } else {
        cameraExtrinsics = solveForParams.slice(3, 6);
      }

      var torsoCalibVals = [0, 0];
      if (args.torsoCalib) {
        torsoCalibVals = (isOneCamera) ? solveForParams.slice(3, 5) :
                                         solveForParams.slice(6, 8);
      }

      let camera2Ground = CalibrationToolkit.makeCamera2Ground(
          snapshot.torso2Ground, torsoCalibVals, snapshot.head2Torso,
          cameraExtrinsics, name);

      var camera2Ground_inv = math.inv(camera2Ground);
      var distanceXTranslation =
          CalibrationToolkit.getFullTransform(undefined, [
            ((isOneCamera) ? solveForParams[5] : solveForParams[8]), 0, 0
          ]);  // solveForParams[5]

      var twoDeeFinal = CalibrationToolkit.transform3Dto2DRPY(
          cameraMatrix[name], camera2Ground_inv,
          math.multiply(distanceXTranslation, snapshot.threeDee));
      var twoDeeFinalSize = math.size(twoDeeFinal).toArray();
      for (var i = 0; i < twoDeeFinalSize[1]; i++) {
        twoDeeArr.push(twoDeeFinal.subset(math.index(0, i)));
        twoDeeArr.push(twoDeeFinal.subset(math.index(1, i)));
      }
    }
  }
  return math.matrix(math.transpose([twoDeeArr]));
};

// get corner coordinates in mm
CalibrationToolkit.getMarkerBoardCornerCoords = function(
    markers, arucoBoard3D) {
  var arucoThreeDeeSet = [new Array(), new Array(), new Array(), new Array()];

  for (const marker of markers) {
    if (arucoBoard3D.markerArr[marker.id]) {  // verify aruco board definition
                                              // contain detected ID
      var threeDeeMarker = new Object();
      Object.assign(
          threeDeeMarker,
          arucoBoard3D.markerArr[marker.id]);  // Assign instead of equate
      threeDeeMarker.x += arucoBoard3D.robotToBoardDistX;
      threeDeeMarker.z += arucoBoard3D.robotToBoardDistZ;
      var markerSizeHalf = arucoBoard3D.markerSize / 2;
      /**
       * Aruco board in RPY coords (nao's coordinates)
       */
      let threeDee = new Array();

      /**
       * TODO automatically transform these
       */

      threeDee = [
        [
          -markerSizeHalf + threeDeeMarker.x,
          +markerSizeHalf + threeDeeMarker.x,
          +markerSizeHalf + threeDeeMarker.x, -markerSizeHalf + threeDeeMarker.x
        ],
        [
          +markerSizeHalf + threeDeeMarker.y,
          +markerSizeHalf + threeDeeMarker.y,
          -markerSizeHalf + threeDeeMarker.y, -markerSizeHalf + threeDeeMarker.y
        ],
        [0, 0, 0, 0], [1, 1, 1, 1]
      ];

      for (let i = 0; i < threeDee[0].length; i++) {  // horiz. concat
        arucoThreeDeeSet[i].push.apply(arucoThreeDeeSet[i], threeDee[i]);
      }
    }
  }
  return arucoThreeDeeSet;
};
/**
 *
 */
CalibrationToolkit.getThreeDeeMarkerCorners = function(markers, arucoBoard3D) {
  var arucoThreeDeeSet = [new Array(), new Array(), new Array(), new Array()];

  for (const marker of markers) {
    if (arucoBoard3D.markerArr[marker.id]) {  // verify aruco board definition
                                              // contain detected ID
      var threeDeeMarker = new Object();
      Object.assign(
          threeDeeMarker,
          arucoBoard3D.markerArr[marker.id]);  // Assign instead of equate
      threeDeeMarker.x += arucoBoard3D.robotToBoardDistX;
      threeDeeMarker.z += arucoBoard3D.robotToBoardDistZ;
      var markerSizeHalf = arucoBoard3D.markerSize / 2;

      /**
       * Aruco board in RPY coords (nao's coordinates)
       */
      let threeDee = new Array();

      /**
       * TODO automatically transform these
       */
      if (arucoBoard3D.orientation == "horizontal") {
        threeDee = [
          [
            +markerSizeHalf + threeDeeMarker.x,
            +markerSizeHalf + threeDeeMarker.x,
            -markerSizeHalf + threeDeeMarker.x,
            -markerSizeHalf + threeDeeMarker.x
          ],
          [
            +markerSizeHalf + threeDeeMarker.y,
            -markerSizeHalf + threeDeeMarker.y,
            -markerSizeHalf + threeDeeMarker.y,
            markerSizeHalf + threeDeeMarker.y
          ],
          [0, 0, 0, 0], [1, 1, 1, 1]
        ];
      } else if (arucoBoard3D.orientation == "vertical") {
        threeDee = [
          [
            threeDeeMarker.x, threeDeeMarker.x, threeDeeMarker.x,
            threeDeeMarker.x
          ],
          [
            +markerSizeHalf + threeDeeMarker.y,
            -markerSizeHalf + threeDeeMarker.y,
            -markerSizeHalf + threeDeeMarker.y,
            markerSizeHalf + threeDeeMarker.y
          ],
          [
            +markerSizeHalf + threeDeeMarker.z,
            +markerSizeHalf + threeDeeMarker.z,
            -markerSizeHalf + threeDeeMarker.z,
            -markerSizeHalf + threeDeeMarker.z
          ],
          [1, 1, 1, 1]
        ];
      }
      for (let i = 0; i < threeDee[0].length; i++) {  // horiz. concat
        arucoThreeDeeSet[i].push.apply(arucoThreeDeeSet[i], threeDee[i]);
      }
    }
  }
  return arucoThreeDeeSet;
};

CalibrationToolkit.makeCamera2HeadCalib = function(
    camName, extrinsics = [0, 0, 0]) {
  let uncalib = CalibrationToolkit.getFullTransform(
      CalibrationToolkit.getRotMat(
          0, CalibrationToolkit.camera2HeadUncalibRot[camName], 0),
      CalibrationToolkit.camera2HeadUncalibPos[camName]);
  /// camera2HeadUncalib * rotX * rotY * rotZ
  /// Ref: ProjectionCamera.cpp
  let extRotation = math.multiply(
      CalibrationToolkit.getRotX(extrinsics[0]),
      math.multiply(
          CalibrationToolkit.getRotY(extrinsics[1]),
          CalibrationToolkit.getRotZ(extrinsics[2])));
  return math.multiply(
      uncalib, CalibrationToolkit.getFullTransform(extRotation));
};

CalibrationToolkit.makeCamera2Ground = function(
    torso2Ground, torsoCalib, head2Torso, camExtrinsics, camName) {

  let camera2Head =
      CalibrationToolkit.makeCamera2HeadCalib(camName, camExtrinsics);
  // calibrated torso matrix
  let torsoCalibMatrix = CalibrationToolkit.getFullTransform(
      math.multiply(
          CalibrationToolkit.getRotY(torsoCalib[0]),
          CalibrationToolkit.getRotX(torsoCalib[1])));

  return math.multiply(
      torso2Ground,
      math.multiply(torsoCalibMatrix, math.multiply(head2Torso, camera2Head)));
};
/**
 * Project marker outline.
 * @param cameraMatrix - cam matrix, 3x3
 * @param camera2Ground_invXYZ
 * @param arucoThreeDeeSet
 * @param strokeColour
 * @param deltaX Shift in x axis from robot to pattern. Used to auto-fix minor
 * issues.
 */
CalibrationToolkit.projectMarkerOutline = function(
    cameraMatrix, camera2Ground_inv, arucoThreeDeeSet, ctx, strokeColour,
    deltaX = 0) {
  let twoDeeOuts;
  if (deltaX == 0) {
    twoDeeOuts = CalibrationToolkit.transform3Dto2DRPY(
        cameraMatrix, camera2Ground_inv, arucoThreeDeeSet);
  } else {
    let distanceXTranslation = CalibrationToolkit.getFullTransform(
        CalibrationToolkit.getRotMat(), [deltaX, 0, 0]);
    twoDeeOuts = CalibrationToolkit.transform3Dto2DRPY(
        cameraMatrix, camera2Ground_inv,
        math.multiply(distanceXTranslation, arucoThreeDeeSet));
  }
  var size = math.size(twoDeeOuts).toArray()[1];
  var projectedCorners = new Array();
  for (var j = 0; j < size; j++) {
    projectedCorners[j] = {
      x: twoDeeOuts.subset(math.index(0, j)),
      y: twoDeeOuts.subset(math.index(1, j))
    };
  }
  ctx.lineWidth = 2;
  ctx.strokeStyle = strokeColour;
  ctx.beginPath();
  let corner = {};
  for (let k = 0; k < projectedCorners.length; k += 4) {
    ctx.moveTo(projectedCorners[k].x, projectedCorners[k].y);  // 1st corner
    for (let j = 0; j < 3; j++) {  // other 3 corners
      corner = projectedCorners[(k + j + 1) % projectedCorners.length];
      ctx.lineTo(corner.x, corner.y);
    }
    ctx.lineTo(
        projectedCorners[k].x, projectedCorners[k].y);  // back to 1st corner
  }
  ctx.stroke();
  ctx.closePath();
};

/**
 * Transformation of camera coordinate system to camera image plane (Similar
 * function in Nao -> this.curCamera2Ground~
 * Reference -
 * http://docs.opencv.org/2.4/modules/calib3d/doc/camera_calibration_and_3d_reconstruction.html
 * @param cameraMatrix camera matrix
 * @param transform related transform
 * @param threeDee 3D point matrix. MUST have 4 rows.
 */
CalibrationToolkit.transform3Dto2D = function(
    cameraMatrix, transform, threeDee) {
  var twoDee = math.multiply(
      cameraMatrix,
      math.multiply(math.matrix(transform).resize([3, 4]), threeDee));
  var twoDeeSize = math.size(twoDee).toArray();
  var twoDeeFinal = math.ones(math.size(twoDee));

  for (var i = 0; i < twoDeeSize[1]; i++) {
    // scaling
    if (twoDee.subset(math.index(2, i)) != 0) {
      var tempResize = math.dotDivide(
          twoDee.subset(math.index(math.range(0, twoDeeSize[0]), i)),
          twoDee.subset(math.index(2, i)));
      twoDeeFinal.subset(
          math.index(math.range(0, twoDeeSize[0]), i), tempResize);
    }
  }
  return twoDeeFinal;
};

/**
 * Transformation of camera coordinate system to camera image plane (Similar
 * function in Nao -> this.curCamera2Ground~
 * Reference -
 * http://docs.opencv.org/2.4/modules/calib3d/doc/camera_calibration_and_3d_reconstruction.html
 * @param cameraMatrix camera matrix
 * @param transform related transform
 * @param threeDee 3D point matrix. MUST have 4 rows.
 */
CalibrationToolkit.transform3Dto2DRPY = function(
    cameraMatrix, transform, threeDee) {
  threeDee = math.matrix(threeDee);

  var transformedThreeDee = math.multiply(transform, threeDee);
  transformedThreeDee =
      math.multiply(CalibrationToolkit.rpy2xyz, transformedThreeDee);
  var threeDeeSize = math.size(transformedThreeDee).toArray();
  var twoDee = math.multiply(
      cameraMatrix, transformedThreeDee.resize([3, threeDeeSize[1]]));
  var twoDeeSize = math.size(twoDee).toArray();
  var twoDeeFinal = math.ones(math.size(twoDee));
  for (var i = 0; i < twoDeeSize[1]; i++) {
    // scaling
    if (twoDee.subset(math.index(2, i)) != 0) {
      var tempResize = math.dotDivide(
          twoDee.subset(math.index(math.range(0, twoDeeSize[0]), i)),
          twoDee.subset(math.index(2, i)));
      twoDeeFinal.subset(
          math.index(math.range(0, twoDeeSize[0]), i), tempResize);
    }
  }
  return twoDeeFinal;
};

/**
 * Camera 3D coord to 2D
 *
 * @param cameraMatrix camera intrinsic matrix
 * @param threeDee 3D point matrix
 */
CalibrationToolkit.cam3Dto2D =
    function(cameraMatrix, threeDee) {
  return CalibrationToolkit.transform3Dto2DRPY(
      cameraMatrix, math.eye(4), threeDee);
}

    /**
     * Get rotation matrix by angles Use convention in NAO
     * @param alpha angle about Roll axis
     * @param beta : pitch
     * @param gamma : yaw
     * @return rotation matrix
     */
    CalibrationToolkit.getRotMat = function(alpha = 0, beta = 0, gamma = 0) {
      var rotX = (alpha == 0) ? math.eye(3) : CalibrationToolkit.getRotX(alpha);
      var rotY = (beta == 0) ? math.eye(3) : CalibrationToolkit.getRotY(beta);
      var rotZ = (gamma == 0) ? math.eye(3) : CalibrationToolkit.getRotZ(gamma);
  return math.matrix(math.multiply(rotZ, math.multiply(rotY, rotX)));
};

CalibrationToolkit.getRotX = function(alpha = 0) {
  return [
    [1, 0, 0], [0, Math.cos(alpha), -Math.sin(alpha)],
    [0, Math.sin(alpha), Math.cos(alpha)]
  ];
};

CalibrationToolkit.getRotY = function(beta = 0) {
  return [
    [Math.cos(beta), 0, Math.sin(beta)], [0, 1, 0],
    [-Math.sin(beta), 0, Math.cos(beta)]
  ];
};

CalibrationToolkit.getRotZ = function(gamma = 0) {
  return [
    [Math.cos(gamma), -Math.sin(gamma), 0],
    [Math.sin(gamma), Math.cos(gamma), 0], [0, 0, 1]
  ];
};

/**
 * get full transform matrix by combining rotM and posV
 * @param rotM
 * @param posV
 * @return full 4x4 matrix
 */
CalibrationToolkit.getFullTransform = function(
    rotM = math.eye(3), posV = [0, 0, 0]) {
  var output = math.subset(math.eye(4, 4), math.index([0, 1, 2], 3), posV);
  return math.subset(output, math.index([0, 1, 2], [0, 1, 2]), rotM);
};

CalibrationToolkit.prototype.drawOriginAxis =
    function(cameraMatrix, transforms, camName) {
  // Draw axis
  var threeDee = [
    [0, 10, 0, 0], [0, 0, 10, 0], [0, 0, 0, 10],
    [1, 1, 1, 1]
  ];  // Origin in respect to each aruco marker's 3D position
  for (var i = 0; i < transforms.length; i++) {
    // var marker = markers[i];
    var transform = transforms[i];
    var twoDeeFinal = CalibrationToolkit.transform3Dto2DRPY(
        cameraMatrix, transform, threeDee);
    /**
     * Draw the 3D axis frame on center of aruco marker.
     */
    this.ctxArr[camName].beginPath();
    this.ctxArr[camName].moveTo(
        twoDeeFinal.subset(math.index(0, 0)),
        twoDeeFinal.subset(math.index(1, 0)));
    this.ctxArr[camName].lineTo(
        twoDeeFinal.subset(math.index(0, 1)),
        twoDeeFinal.subset(math.index(1, 1)));
    this.ctxArr[camName].strokeStyle = "red";
    this.ctxArr[camName].stroke();
    this.ctxArr[camName].closePath();
    this.ctxArr[camName].beginPath();
    this.ctxArr[camName].moveTo(
        twoDeeFinal.subset(math.index(0, 0)),
        twoDeeFinal.subset(math.index(1, 0)));
    this.ctxArr[camName].lineTo(
        twoDeeFinal.subset(math.index(0, 2)),
        twoDeeFinal.subset(math.index(1, 2)));
    this.ctxArr[camName].strokeStyle = "green";
    this.ctxArr[camName].stroke();
    this.ctxArr[camName].closePath();
    this.ctxArr[camName].beginPath();
    this.ctxArr[camName].moveTo(
        twoDeeFinal.subset(math.index(0, 0)),
        twoDeeFinal.subset(math.index(1, 0)));
    this.ctxArr[camName].lineTo(
        twoDeeFinal.subset(math.index(0, 3)),
        twoDeeFinal.subset(math.index(1, 3)));
    this.ctxArr[camName].strokeStyle = "blue";
    this.ctxArr[camName].stroke();
    this.ctxArr[camName].closePath();
  }
}

    /**
     * Draw aruco marker ID
     *
     * @param markers aruco marker array
     */
    CalibrationToolkit.prototype.drawId = function(markers) {
  var corners, corner, x, y, i, j;
  this.ctx.strokeStyle = "blue";
  this.ctx.lineWidth = 1;

  for (i = 0; i !== markers.length; ++i) {
    corners = markers[i].corners;
    x = Infinity;
    y = Infinity;
    for (j = 0; j !== corners.length; ++j) {
      corner = corners[j];

      x = Math.min(x, corner.x);
      y = Math.min(y, corner.y);
    }
    this.ctx.strokeText(markers[i].id, x, y)
  }
};

/**
 * Return a 3x3 camera matrix upon recieving centers and focal lengths
 */
CalibrationToolkit.getCameraMatrix = function(
    cam_fc, cam_cc, width = 1, height = 1) {
  return math.matrix([
    [cam_fc[0] * width, 0, cam_cc[0] * width],
    [0, cam_fc[1] * height, cam_cc[1] * height], [0, 0, 1]
  ]);
};

CalibrationToolkit.getMatlabMatStr = function(name, mat) {
  var data = math.matrix(mat).toArray();
  //     console.log(data);
  if (!name) {
    name = "mat";
  }
  var output = name + "= [";
  var size = math.size(mat);
  size = (size instanceof Array) ? size : math.size(mat).toArray();
  for (var i = 0; i < size[0]; i++) {
    for (var j = 0; j < size[1]; j++) {
      output += (data[i][j]).toString() + " ";
    }
    output += ";\n";
  }
  output += "]\n";
  return output;
};
/**
 * Print a matrix in Octave/ Matlab compatible form on console.
 *
 * @param mat Matrix object. Accept 2D, etc array elements, MathJS mat type.
 * @param name Variable name to be printed with the mat.
 */

CalibrationToolkit.printMat = function(name, mat) {
  console.log(CalibrationToolkit.getMatlabMatStr(name, mat));
};

module.exports = CalibrationToolkit;
