var AR = require('js-aruco').AR;
var POS = require('js-aruco').POS2;
var util = require('./ui').util;
var DebugDisplay = require('./debugDisplay');
var _ = require('underscore');
var Graph = require('./graph');
var configMan = require('./connection').configMan;
var CalibrationToolkit = require('./calibration');
var math = require('mathjs');
var debugMan = require('./connection').debugMan;

/**
 * Created by Darshana 27.11.2016
 * Using ArUco markers to calibrate intrinsic and extrinsic parameters of the
 * cameras
 */
function CalibrationEditor() {
    this.init.apply(this, arguments);
}

util.inherits(CalibrationEditor, DebugDisplay);
CalibrationEditor.defaultConfig = {
    title: '',
    width: 500,
    height: 101,
    bufferSize: 100,
    minValue: -2,
    maxValue: 2,
    autoScale: true
};

CalibrationEditor.show = function () {
    return new CalibrationEditor();
};

CalibrationEditor.expectedKeys = [['object'], ['[object]']];
CalibrationEditor.cameraNames = ["top", "bottom"];
CalibrationEditor.threeDeeMatRows = 4;
CalibrationEditor.prototype.init = function (config) {
    DebugDisplay.prototype.init.call(this, config);

    /**
     * Dynamic script addition
     * TODO TESTING !!
     */
    // CalibrationEditor.addScript("/libs/js-aruco/aruco.js");
    // CalibrationEditor.addScript("/libs/js-aruco/cv.js");
    // CalibrationEditor.addScript("/libs/js-aruco/posit2.js");
    // CalibrationEditor.addScript("/libs/js-aruco/svd.js");
    // CalibrationEditor.addScript("/libs/math.min.js");
    // CalibrationEditor.addScript("/libs/calibration.js");
    // CalibrationEditor.addScript("/libs/fminsearch.js");

    this.arucoBoard3D = config.arucoBoard3D;
    // config
    this.config = _.defaults(config || {}, Graph.defaultConfig);
    this.config.bufferSize = parseInt(this.config.bufferSize);
    this.config.minValue = parseFloat(this.config.minValue);
    this.config.maxValue = parseFloat(this.config.maxValue);
    this.showMinMaxLegend = (this.config.autoScale) ? true : false;

    var captureAllBtn = this._capImage = document.createElement('div');
    captureAllBtn.style.marginRight = "5px";
    captureAllBtn.className = 'btn';
    captureAllBtn.textContent = 'Capture';
    captureAllBtn.addEventListener(
        'click', _.bind(this.onCaptureAllClick, this), false);
    this.head.appendChild(captureAllBtn);

    var calibrateBtn = this._capImage = document.createElement('div');
    calibrateBtn.style.marginRight = "5px";
    calibrateBtn.className = 'btn';
    calibrateBtn.textContent = 'Calibrate';
    calibrateBtn.addEventListener(
        'click', _.bind(this.onCalibrateClick, this), false);
    this.head.appendChild(calibrateBtn);

    // Upload/ save Buttons. Placed above image.
    // getEventListeners().click.forEach((e)=>{e.remove()})
    this.uploadPanel = document.createElement('div');
    this.wrapper.appendChild(this.uploadPanel);

    this.captureImg = false;
    this.captureImg2 = false;
    this.captureMat = false;
    this.doIntrinsic = false;
    this.doExtrinsic = true;
    {
        // let tempDiv = document.createElement("div");
        // this.saveIntrinsic = document.createElement("INPUT");
        // this.saveIntrinsic.type = "checkbox";
        // // this.saveIntrinsic.setAttribute("class", "inlineInput");
        // this.saveIntrinsic.checked = false;
        // this.saveIntrinsic.style.display = "inline";
        // this.saveIntrinsic.style.marginRight = "15px";
        // };

        // let label1 = document.createElement("label");
        // label1.innerText = "Intrinsic";
        // label1.appendChild(this.saveIntrinsic);
        // tempDiv.appendChild(label1);

        // this.saveExtrinsic = document.createElement("INPUT");
        // this.saveExtrinsic.type = "checkbox";
        // this.saveExtrinsic.style.display = "inline";
        // // this.saveIntrinsic.setAttribute("class", "inlineInput");
        // this.saveExtrinsic.checked = true;

        // let label2 = document.createElement("label");
        // label2.innerText = "Extrinsic";
        // label2.appendChild(this.saveExtrinsic);
        // tempDiv.appendChild(label2);
        // tempDiv.style.marginRight = "10px";
        // tempDiv.style.cssFloat = "right";
        // this.head.appendChild(tempDiv);
    }

    this.currentCameraName = "top";

    // Get config for Brain.Projection - calibration will start with these values
    this.getHeadProjectionConfig((err, hpc) => {
        if (err) return;
        this.headProjectionConfig = hpc;
    });

    this.projectMarkerButton = document.createElement('button');
    this.projectMarkerButton.className = 'btn';
    this.projectMarkerButton.textContent = "project Markers";
    this.projectMarkerButton.style.marginRight = "5px";
    this.projectMarkerButton.addEventListener(
        'click', _.bind(this.projectMarkers, this), false);
    this.head.appendChild(this.projectMarkerButton);


    this.camSelect = document.createElement('select');
    this.head.appendChild(this.camSelect);
    this.camSelect.addEventListener(
        'change', _.bind(this.onCamSelectorChange, this), false);
    this.camSelect.style.width = "100px";
    this.camSelect.style.cssFloat = "right";
    this.camSelect.style.marginRight = "10px";
    for (const key of CalibrationEditor.cameraNames) {
        var elem = document.createElement('option');
        elem.value = key;
        elem.appendChild(document.createTextNode(key));
        this.camSelect.appendChild(elem);
        if (key == this.currentCameraName) {
            elem.selected = true;
        }
    }

    //<pre> tags to nicely print matrices
    this.preformattedArea = document.createElement('pre');
    this.uploadPanel.appendChild(this.preformattedArea);

    {
        this.configHtml = document.createElement('div');
        var configHtmlString = `
      <button id="configHead" style="margin:5px">
        Show/ Hide Config
      </button>
      <div id="configBody" hidden>
        <div>
          <label>Intrinsic</label>
          <input class="intrinsicGroup inlineInput" style="display:inline" type="checkbox" name="intrinsicGroup" value="top">
          <label for="top">top</label>
          <input class="intrinsicGroup inlineInput" style="display:inline" type="checkbox" name="intrinsicGroup" value="bottom">
          <label for="bottom">bottom</label>
        </div>
        <div>
          <label>Extrinsic</label>
          <input class="extrinsicGroup inlineInput" style="display:inline" type="checkbox" name="extrinsicGroup" value="top" checked="true">
          <label for="top">top</label>
          <input class="extrinsicGroup inlineInput" style="display:inline" type="checkbox" name="extrinsicGroup" value="bottom" checked="true">
          <label for="bottom">bottom</label>
        </div>
        <div>
          <label> Torso Calibration</label>
          <input id="torsoCheck" class="inlineInput" type="checkbox" name="torsoCheck" value="torsoCalibrate">
      </div>
    `;
        this.configHtml.innerHTML = configHtmlString;
        this.configHtml.id = "config";
        this.configHtml.className = 'inputInfoGroup';
        this.wrapper.appendChild(this.configHtml);
        document.getElementById("configHead")
            .addEventListener('click', _.bind(function () {
                if (document.getElementById("configBody").hasAttribute('hidden')) {
                    document.getElementById("configBody").removeAttribute('hidden')
                } else {
                    document.getElementById("configBody").setAttribute("hidden", true);
                }
            }, this), false);
    }
    // head joint move
    {
        // list of head motions
        this.headMotionArr = config.headMotionArr;

        // onHeadMotionEnableChange
        this.headMotionPanel = document.createElement('div');
        this.headMotionPanel.style = {
            cssFloat: "right",
            width: "100px",
            display: "block"
        };
        this.headMotionPanel.className = 'inputInfoGroup';
        this.headMotionEnabled = document.createElement("INPUT");
        this.headMotionEnabled.type = "checkbox";
        this.headMotionEnabled.style.display = "inline";
        this.headMotionEnabled.checked = false;
        this.headMotionEnabled.addEventListener(
            'change', _.bind(this.onHeadMotionEnableChange, this), false);

        let label1 = document.createElement("label");
        label1.innerText = "Head motion";
        label1.appendChild(this.headMotionEnabled);
        this.headMotionPanel.appendChild(label1);

        this.headPitchInput = document.createElement('input');
        this.headYawInput = document.createElement('input');
        this.headPitchInput.disabled = true;
        this.headYawInput.disabled = true;
        this.headPitchInput.style.width = "80px";
        this.headPitchInput.style.marginRight = "10px";
        this.headYawInput.style.width = "80px";
        this.headYawInput.style.marginRight = "10px";

        let label2 = document.createElement("label");
        label2.innerText = "Yaw";


        var headMotionButton = this.moveHeadBtn = document.createElement('button');
        headMotionButton.style.marginRight = "5px";
        headMotionButton.className = 'btn';
        headMotionButton.textContent = 'Move Head';
        headMotionButton.disabled = true;
        headMotionButton.addEventListener(
            'click', _.bind(this.onHeadMotionButton, this), false);

        var headMotionMultiButton = this.moveHeadMultiBtn =
            document.createElement('button');
        headMotionMultiButton.style.marginRight = "5px";
        headMotionMultiButton.className = 'btn';
        headMotionMultiButton.textContent = 'Move HeadLoop';
        headMotionMultiButton.disabled = true;
        headMotionMultiButton.addEventListener(
            'click', _.bind(this.onHeadMotionMultiButton, this), false);

        var headMotionStopButton = this.stopHeadBtn =
            document.createElement('button');
        headMotionStopButton.style.marginRight = "5px";
        headMotionStopButton.className = 'btn';
        headMotionStopButton.textContent = 'Stop HeadLoop';
        headMotionStopButton.disabled = true;

        this.headMotionPanel.appendChild(this.headPitchInput);
        this.headMotionPanel.appendChild(label2);
        this.headMotionPanel.appendChild(this.headYawInput);
        this.headMotionPanel.appendChild(headMotionButton);
        this.headMotionPanel.appendChild(headMotionMultiButton);
        this.headMotionPanel.appendChild(headMotionStopButton);

        var downloadBtn = document.createElement('button');
        downloadBtn.textContent = 'Download snapshots';
        downloadBtn.addEventListener('click', _.bind(function () {
            let snap = this.snapshots["top"][this.snapshots["top"].length - 1];
            console.log(snap);
            CalibrationToolkit.printMat("head2Torso", snap.head2Torso);
            CalibrationToolkit.printMat("torso2Ground", snap.torso2Ground);
            console.log(snap.threeDee);
            CalibrationToolkit.printMat("threeDee", snap.threeDee);

            CalibrationToolkit.printMat("twoDeeMat", snap.twoDeeMat);
            CalibrationToolkit.printMat("camera2Ground_inv", snap.camera2Ground_inv);
            let camMatrix = CalibrationToolkit.getCameraMatrix(
                this.headProjectionConfig["top_fc"],
                this.headProjectionConfig["top_cc"], this.canvasArr["top"].width,
                this.canvasArr["top"].height);
            CalibrationToolkit.printMat(
                "twoDeeGen", CalibrationToolkit.transform3Dto2DRPY(
                    camMatrix, snap.camera2Ground_inv, snap.threeDee));
            CalibrationToolkit.printMat("cameraMatrix", camMatrix);

            CalibrationEditor.download(
                "snapshots_" + (new Date().getTime()) + '.json',
                JSON.stringify(this.snapshots || {}, null, 4), 'application/json');
        }, this), false);
        this.headMotionPanel.appendChild(downloadBtn);
        this.wrapper.appendChild(this.headMotionPanel);
    }
    let tempDiv = document.createElement('div');

    // Canvas
    this.canvas = document.createElement('canvas');
    this.canvas.style.position = 'relative';
    this.canvas.style.display = 'inline-block';
    this.canvas.width = 640;
    this.canvas.imageratioW2H = 640 / 480;
    this.canvas.height = this.canvas.width / this.canvas.imageratioW2H;
    // this.canvas.addEventListener('click', _.bind(this.onCanvasClick, this),
    // false);
    this.ctx = this.canvas.getContext('2d');
    tempDiv.appendChild(this.canvas);

    this.onImgUpdate = _.bind(this.onImgUpdate, this);
    this.onImgUpdate2 = _.bind(this.onImgUpdate2, this);
    this.onUpdate = _.bind(this.onUpdate, this);

    // Canvas
    this.canvas2 = document.createElement('canvas');
    this.canvas2.style.position = 'relative';
    this.canvas2.style.display = 'inline-block';
    this.canvas2.width = 640;
    this.canvas2.imageratioW2H = 640 / 480;
    this.canvas2.height = this.canvas2.width / this.canvas2.imageratioW2H;
    // this.canvas.addEventListener('click', _.bind(this.onCanvasClick, this),
    // false);
    this.ctx2 = this.canvas2.getContext('2d');

    tempDiv.appendChild(this.canvas2);
    this.wrapper.appendChild(tempDiv);

    this.canvasArr = {"top": this.canvas, "bottom": this.canvas2};
    this.ctxArr = {"top": this.ctx, "bottom": this.ctx2};
    // MarkerList
    // this.markerListDiv = document.createElement('div');
    // this.wrapper.appendChild(this.markerListDiv);

    // get other subscriptions -> views/calibration.json
    this.subscribe(config.keys, config.mappingFct);

    // track last update between cam2ground & images TODO implement the logic
    this.lastUpdate = 0;

    // Initialize matrices TODO implement things manually later?
    this.camera2Ground_inv = new Object();  // math.zeros(4, 4);
    // this.camera2Ground = new Object(); //math.zeros(4, 4);
    this.torso2Ground = CalibrationToolkit.getFullTransform();
    this.head2Torso = CalibrationToolkit.getFullTransform();

    this.torso2GroundDot = CalibrationToolkit.getFullTransform();
    this.head2TorsoDot = CalibrationToolkit.getFullTransform();

    this.matrixLastUpdateTime = new Date();
    this.imageLastUpdateTime = new Date();
    //         this.camera2Ground=math.zeros(4, 4);
    this.camera_cc = math.zeros(3, 1);

    this.currentImage = null;
    this.marker = {};

    /**
     * Multi image capture
     */
    this.snapshots = {top: new Array(), bottom: new Array()};
    /**
     * snapshots:{
   *      "top":[
   *              markerPoints:[], OR give markers. In future, give 2D points
   * and corresponding 3D points only.
   *              head2Torso:{},
   *              torso2Ground:{},
   *              camera2Ground_inv:[]
   *      ],
   *      "bottom":[]
   * }
     */
    // Subscription to get images.
    this.imgSubscription = debugMan.subscribeImage(
        "Brain.ImageReceiver.top_image", this.onImgUpdate);
    this.imgSubscription2 = debugMan.subscribeImage(
        "Brain.ImageReceiver.bottom_image", this.onImgUpdate2);

    this.headProjectionConfig = null;
    // if view/calibration directly loaded, the configmanager does not work**
    // Therefore redirect to home.
};

CalibrationEditor.prototype.projectMarkers = function () {
    // Get config for Brain.Projection - calibration will start with these values
    this.getHeadProjectionConfig((err, hpc) => {
        if (err) return;
        this.headProjectionConfig = hpc;
    })
    ;

    for (let name of CalibrationEditor.cameraNames) {
        let cameraMatrix = CalibrationToolkit.getCameraMatrix(
            this.headProjectionConfig[name + "_fc"],
            this.headProjectionConfig[name + "_cc"], this.canvasArr[name].width,
            this.canvasArr[name].height);
        let snapshot = this.snapshots[name].slice(-1)[0];
        if (!snapshot) {
            continue;
        }
        // let camera2Ground_invXYZ =
        //     math.multiply(CalibrationToolkit.rpy2xyz,
        //     snapshot.camera2Ground_inv);
        let arucoThreeDeeSet = CalibrationToolkit.getThreeDeeMarkerCorners(
            snapshot.markers, this.arucoBoard3D[name]);
        CalibrationToolkit.projectMarkerOutline(
            cameraMatrix, snapshot.camera2Ground_inv, arucoThreeDeeSet,
            this.ctxArr[name], "green", 0);
    }
};

CalibrationEditor.prototype.getHeadProjectionConfig = function (cb) {
    const headProjectionMountPoint = 'Brain.Projection';
    configMan.getMounts((mountPoints) => {

        // Verify that mountPoint actually exists
        if (
            !mountPoints.some((mp) => mp.key == headProjectionMountPoint
            )) {
            alert('Redirecting to Home; please use open view -> calibration');
            window.location.replace('/');
            return;
        }

        configMan.getKeys(headProjectionMountPoint, (keys) => {
                const headProjectionConfig = {};
                for (const item of keys) {
                    headProjectionConfig[item.key] = item.value;
                }
                cb(null, headProjectionConfig);
            },
            false
        )
        ;
    })
    ;
};

/**
 * Event handler for camera selecter change event
 * Will change image subscription
 * TODO Maybve in future, we'll have to get both images at the same time
 *
 * 07.06.2017 Disabled camera selector event -> Will be capturing both cameras
 * now.
 */

CalibrationEditor.prototype.onCamSelectorChange = function (key) {
    // if (this.camSelect.value == "top" || this.camSelect.value == "bottom") { //
    // sanity check
    //     console.log("changed cam to", this.camSelect.value, key);
    //     debugMan.unsubscribe(this.imgSubscription);
    //     this.imgSubscription = debugMan.subscribeImage("Brain.ImageReceiver." +
    //     this.camSelect.value + "_image", this.onImgUpdate);
    //     this.currentCameraName = this.camSelect.value;
    // }
};


CalibrationEditor.prototype.onHeadMotionMultiButton = function () {

    this.stopHeadBtn.disabled = false;
    this.moveHeadMultiBtn.disabled = true;

    let index = 0;
    let len = this.headMotionArr.length;
    this.timedHeadMotion = -1;

    var stopInterval = function () {
        clearInterval(this.timedHeadMotion);
        this.stopHeadBtn.disabled = true;
        this.moveHeadMultiBtn.disabled = false;
    }.bind(this);

    this.stopHeadBtn.addEventListener('click', _.bind(function () {
        stopInterval();

    }, this), false);

    this.timedHeadMotion = setInterval(function (configMan) {
        // if (index != 0) {
        this.onCaptureAllClick();
        // }
        if (!(index < len)) {
            stopInterval();
            return;
        }

        let motion = this.headMotionArr[index];
        // console.log(motion);
        // console.log(index, headMotionArr);
        let yaw = parseFloat(motion.yaw);
        let pitch = parseFloat(motion.pitch);

        if (!isNaN(yaw) && !isNaN(pitch)) {  // sanity check
            if (Math.abs(yaw) <= 119.5 && pitch >= -38.5 && pitch <= 29.5) {
                configMan.set("Brain.BehaviorModule", "calibrationHeadPitch", pitch);
                configMan.set("Brain.BehaviorModule", "calibrationHeadYaw", yaw);
            } else {
                alert(
                    "Values Out of range. Yaw : -119.5 to 119.5, pitch : -38.5 to 29.5");
            }
        } else {
            console.log("nan", yaw, pitch);
        }
        index++;
    }.bind(this, configMan, index, len, this.headMotionArr), 1000);


    // for (motion of this.headMotionArr) {
    // let yaw = parseFloat(motion.yaw);
    // let pitch = parseFloat(motion.pitch);

    // if (!isNaN(yaw) && !isNaN(pitch)) { // sanity check
    //     if (yaw >= -2.0857 && yaw <= 2.0857 && pitch >= -0.6720 && pitch <=
    //     0.5149) {
    //         configMan.set("Brain.BehaviorModule", "calibrationHeadPitch",
    //         pitch);
    //         configMan.set("Brain.BehaviorModule", "calibrationHeadYaw", yaw);
    //     } else {
    //         console.log("Values Out of range. Yaw : -2.0857 to 2.0857, pitch :
    //         -0.6720 to 0.5149");
    //     }
    // } else {
    //     console.log("nan", yaw, pitch);
    // }
    // }
};

CalibrationEditor.prototype.onHeadMotionButton = function () {
    let yaw = parseFloat(this.headYawInput.value);
    let pitch = parseFloat(this.headPitchInput.value);

    if (!isNaN(yaw) && !isNaN(pitch)) {  // sanity check
        if (Math.abs(yaw) <= 119.5 && pitch >= -38.5 && pitch <= 29.5) {
            configMan.set("Brain.BehaviorModule", "calibrationHeadPitch", pitch);
            configMan.set("Brain.BehaviorModule", "calibrationHeadYaw", yaw);
        } else {
            alert(
                "Values Out of range. Yaw : -119.5 to 119.5, pitch : -38.5 to 29.5");
        }
    } else {
        console.log("nan", yaw, pitch);
    }
};

CalibrationEditor.prototype.onHeadMotionEnableChange = function () {
    configMan.set(
        "Brain.BehaviorModule", "isCameraCalibration",
        this.headMotionEnabled.checked);
    this.moveHeadBtn.disabled = !this.headMotionEnabled.checked;
    this.headYawInput.disabled = !this.headMotionEnabled.checked;
    this.headPitchInput.disabled = !this.headMotionEnabled.checked;
    this.moveHeadMultiBtn.disabled = !this.headMotionEnabled.checked;
};

/**
 * on recieving debug messages. See views/calibration.json for specifics
 */

CalibrationEditor.prototype.onUpdate = function (val) {
    let updateTime = new Date();
    let dt = (updateTime.getTime() - this.matrixLastUpdateTime.getTime()) *
        1000;  // dt in seconds
    if (this.captureMat == false) {
        return;
    }

    // camera2Ground_inv build.
    for (let cameraName of CalibrationEditor.cameraNames) {
        let val1 = val.camera2Ground_inv[cameraName];
        val1.posV = math.multiply(val1.posV, 1000);
        this.camera2Ground_inv[cameraName] = CalibrationToolkit.getFullTransform(
            math.transpose(val1.rotM),
            val1.posV
        );
//        this.camera2Ground_inv[cameraName] = CalibrationToolkit.getFullTransform(
//            math.transpose(val1.rotM), val1.posV);
    }
    // camera2Ground build
    // {
    //     let val1 = val.camera2Ground[this.currentCameraName];
    //     val1.posV = math.multiply(val1.posV, 1000);
    //     this.camera2Ground[this.currentCameraName] =
    //     CalibrationToolkit.getFullTransform(math.transpose(val1.rotM),
    //     val1.posV);
    //     //             console.log(key,"cam2ground inv. gen.",
    //     math.inv(math.matrix(val1.rotM)));
    // }
    // torso2Ground build
    {
        let val1 = val.torso2Ground;  // TODO attaching camera name has to be fixed
                                      // from Nao's side as well.
        console.log("rotM", val1.rotM);
        let tempMat = CalibrationToolkit.getFullTransform(
            math.transpose(val1.rotM),
            val1.posV
        );
//        let tempMat = CalibrationToolkit.getFullTransform(
//            math.transpose(val1.rotM), val1.posV);

        // let diff = math.subtract(tempMat, this.torso2Ground);
        // this.
        this.torso2Ground = tempMat;
        //             console.log(key,"cam2ground inv. gen.",
        //             math.inv(math.matrix(val1.rotM)));
    }
    // head2Torso
    {
        let val1 = val.head2Torso;  // TODO attaching camera name has to be fixed
                                    // from Nao's side as well.
        this.head2Torso = CalibrationToolkit.getFullTransform(
            math.transpose(val1.rotM),
            val1.posV
        );
    }
    console.log('loaded kinematics');
    console.log(this.torso2Ground);
    console.log(this.camera2Ground_inv);
    // TODO check time diff between last matrix and image
    this.captureMat = false;
};

/**
 * Got an update for the image subscription, load image and proceed
 * 04.03.2017 Added loadingImg binding as per suggestion of @robofish
 */

CalibrationEditor.prototype.onImgUpdate = function (image) {
    if (this.captureImg == false) {
        return;
    }
    var loadingImg = new Image();
    loadingImg.src = '/image/' + image + '?' + new Date().getTime();
    this.lastUpdate = new Date().getTime();
    loadingImg.addEventListener(
        'load', _.bind(this.onImageLoad, this, loadingImg, "top"));
    this.captureImg = false;
};

CalibrationEditor.prototype.onImgUpdate2 = function (image) {
    if (this.captureImg2 == false) {
        return;
    }
    var loadingImg = new Image();
    loadingImg.src = '/image/' + image + '?' + new Date().getTime();
    this.lastUpdate = new Date().getTime();
    loadingImg.addEventListener(
        'load', _.bind(this.onImageLoad, this, loadingImg, "bottom"));
    this.captureImg2 = false;
};

CalibrationEditor.markerIds = ['1', '2', '3', '4'];
CalibrationEditor.lines = [['1', '2'], ['2', '3'], ['3', '4'], ['4', '1']];

CalibrationEditor.prototype.draw = function () {
    this.canvas.width = this.currentImage.width;
    this.canvas.height = this.currentImage.height;
    this.ctx.drawImage(
        this.currentImage, 0, 0, this.currentImage.width,
        this.currentImage.height);

    this.ctx.fillStyle = 'red';
    this.ctx.strokeStyle = 'red';
    this.ctx.lineWidth = 1;

    for (const markerId in this.marker) {
        const point = this.marker[markerId];
        this.ctx.beginPath();
        this.ctx.arc(point.x, point.y, 2, 0, 2 * Math.PI);
        this.ctx.fill();
    }

    for (const line of CalibrationEditor.lines) {
        if (!this.marker.hasOwnProperty(line[0]) ||
            !this.marker.hasOwnProperty(line[1]))
            continue;
        const from = this.marker[line[0]];
        const to = this.marker[line[1]];
        this.ctx.beginPath();
        this.ctx.moveTo(from.x, from.y);
        this.ctx.lineTo(to.x, to.y);
        this.ctx.stroke();
    }

    this.refreshMarkerList();
};

CalibrationEditor.prototype.refreshMarkerList = function () {
    this.markerListDiv.innerHTML = '';
    for (const markerId of CalibrationEditor.markerIds) {
        const point = this.marker[markerId];
        const div = document.createElement('div');
        div.className = 'calibMarker';
        div.textContent = markerId;
        if (point) {
            div.textContent += ` (${point.x}, ${point.y})`;
        }
        if (markerId === this.currentMarker) {
            div.classList.add('active');
        }
        div.addEventListener('click', () => {
            this.currentMarker = markerId;
            this.refreshMarkerList();
        })
        ;
        this.markerListDiv.appendChild(div);
    }
};

CalibrationEditor.prototype.onCanvasClick = function (e) {
    if (!this.currentMarker) return;
    const point = {x: e.layerX, y: e.layerY};
    this.marker[this.currentMarker] = point;
    this.draw();
};


/**
 * Capture image and do Extrinsic button
 */
CalibrationEditor.prototype.onCaptureAllClick = function () {
    // if (!this.saveIntrinsic.checked && !this.saveExtrinsic.checked) {
    //   alert("Select Calibration Type");
    //   return;
    // }

    if (!this.headProjectionConfig) {
        this.getHeadProjectionConfig((err, hpc) => {
            if (err) return;
            this.headProjectionConfig = hpc;
            this.onCaptureAllClick();
        })
        ;
    }

    // this.doIntrinsic = this.saveIntrinsic.checked;
    // this.doExtrinsic = this.saveExtrinsic.checked;
    this.captureMat = true;
    this.captureImg = true;
    this.captureImg2 = true;
};

/**
 * On image load
 * Will init Aruco and search for markers.
 * NEW sent this code to calibration.js for modularity
 * snapshots:[
 *      {
 *          markerPoints:[], OR give markers. In future, give 2D points and
 * corresponding 3D points only.
 *          head2Torso:{},
 *          torso2Ground:{},
 *          camera2Ground_inv:{}
 *      }
 * ]
 *
 * 04.03.2017 Added loadingImg as a parameter as per suggestion of @robofish
 * 07.06.2017 Added dual camera support. No longer rely on
 * this.currentCameraName
 */
CalibrationEditor.prototype.onImageLoad = function (loadingImg, cameraName) {
    this.currentImage = loadingImg;
    cameraName = (cameraName) ? cameraName : this.currentCameraName;
    let snapshot = {
        head2Torso: this.head2Torso,
        torso2Ground: this.torso2Ground,
        camera2Ground_inv: this.camera2Ground_inv[cameraName]
    };
    console.log(this.camera2Ground_inv);
    // adjust canvas and load image to canvas
    this.canvasArr[cameraName].width = loadingImg.width;
    this.canvasArr[cameraName].height = loadingImg.height;
    this.ctxArr[cameraName].drawImage(
        loadingImg, 0, 0, this.canvasArr[cameraName].width,
        this.canvasArr[cameraName].height);
    imageData = this.ctxArr[cameraName].getImageData(
        0, 0, this.canvasArr[cameraName].width, this.canvasArr[cameraName].width);


    //=== detect aruco markers ===

    let detector = new AR.Detector();
    const tempMarkers = detector.detect(imageData);

    // transforms for each aruco marker -
    // transforms from aruco 3D points to
    // camera. NOTE NOT in Roll Pitch Yaw
    // config*
    snapshot.transforms = new Array();
    snapshot.twoDeePoints = new Array();
    snapshot.twoDeeMat = [[], []];
    var ids = new Array();
    var markers = new Array();
    for (var i = 0; i < tempMarkers.length; i++) {
        var marker = tempMarkers[i];
        // removing duplicate ID's
        if ((marker.id in ids)) {
            continue;
        }
        ids[marker.id] = true;
        markers.push(marker);
        // draw id
        // this.ctx.strokeStyle = "blue";
        // this.ctx.lineWidth = 1;
        var corners = marker.corners;
        // centering corner coords to center of canvas -> bring origin from
        // left
        // top corner to center of image
        /**
         0 = -40  40  0
         1 =  40  40  0
         2 =  40 -40  0
         3 = -40 -40  0
         */
        var newCorners = new Array();
        for (let cornerIndex = 0; cornerIndex < corners.length; cornerIndex++) {
            let corner = corners[cornerIndex];
            // UMath.sing a new object and object copy as ordinary assignment
            // will
            // only refer to the original object
            var newCorner = new Object();
            Object.assign(newCorner, corner);  // Assign ** Important
            newCorners.push(newCorner);

            // push both x and y to the independant variable list fed for
            // iterative optimizer
            snapshot.twoDeeMat[0].push(newCorner.x);
            snapshot.twoDeeMat[1].push(newCorner.y);
            snapshot.twoDeePoints.push(newCorner.x);
            snapshot.twoDeePoints.push(newCorner.y);
            corner.x = corner.x - (this.canvasArr[cameraName].width / 2);
            corner.y = corner.y - (this.canvasArr[cameraName].height / 2);
        }

        /**
         * Pose estimation Only used for intrinsics
         */
        var posit = new POS.Posit(
            this.arucoBoard3D[cameraName].markerSize || 50,
            this.canvasArr[cameraName].width);  // marker size (default =80)
        // CalibrationToolkit,getMarkerBoardCornerCoords();
        // CalibrationToolkit.
        // posit.model =
        var pose = posit.pose(corners);

        // Converting pose object into a 4x4 matrix format -> [R | T]
        pose.bestRotation[0][3] = pose.bestTranslation[0];
        pose.bestRotation[1][3] = pose.bestTranslation[1];
        pose.bestRotation[2][3] = pose.bestTranslation[2];
        pose.bestRotation[3] = [0, 0, 0, 1];

        var transform =
            math.multiply(CalibrationToolkit.xyz2rpy, pose.bestRotation);
        // now rotM is a 4x4, transform
        // is the transformation matrix
        // that define cam pose W.R.T.
        // aruco marker
        // marker.transform = transform;
        // transform.id = marker.id;
        // transdform to RPY format of nao
        snapshot.transforms.push(transform);
    }
    snapshot.markers = markers;
    // console.log(this);
    // console.log(this.arucoBoard3D[cameraName]);
    snapshot.threeDee = CalibrationToolkit.getThreeDeeMarkerCorners(
        markers, this.arucoBoard3D[cameraName]);

    if (snapshot.threeDee.length != CalibrationEditor.threeDeeMatRows) {
        throw "threeDee Point lacks dimensions" + threeDeePoint.length;
    }

    // 3D point count per snapshot
    snapshot.threeDeePointCount = snapshot.threeDee[0].length;
    snapshot.camera2Ground_invXYZ = math.multiply(
        CalibrationToolkit.rpy2xyz, math.inv(snapshot.camera2Ground_inv));

    console.log(
        "Detection End " + cameraName + ", marker count : " + markers.length);

    this.snapshots[cameraName].push(snapshot);
    console.log(
        "cam " + cameraName + " snap len: " + this.snapshots[cameraName].length);
};

CalibrationEditor.prototype.onCalibrateClick = function () {
    // TODO: do calls to calibration library here.
    // TODO: only do the following after markers have been set and "Calibrate" is clicked.\
    let intrinsicGrp = document.getElementsByClassName("intrinsicGroup");
    let extrinsicGrp = document.getElementsByClassName("extrinsicGroup");
    this.doIntrinsic = {};
    this.doExtrinsic = {};
    this.doTorso = document.getElementById("torsoCheck").checked;
    for (let val of intrinsicGrp) {
        this.doIntrinsic[val.value] = val.checked;
    }
    for (let val of extrinsicGrp) {
        this.doExtrinsic[val.value] = val.checked;
    }

    var args = new Object();
    args.cameraNames = CalibrationEditor.cameraNames;
    // args.canvas = this.canvas;
    // args.context = this.ctx;
    args.ctxArr = this.ctxArr;
    args.canvasArr = this.canvasArr;
    args.projectionConfig = this.headProjectionConfig;
    args.arucoBoard3D = this.arucoBoard3D;

    var calibTools = new CalibrationToolkit(args);

    // Call snapshot and attach the callback function.
    calibTools.extrinsicMultiImage(
        {
            snapshots: this.snapshots,
            doExtrinsic: this.doExtrinsic,
            doIntrinsic: this.doIntrinsic,
            doTorso: this.doTorso
        },
        function (values) {  // callback
            // console.log("callback", JSON.stringify(val, null, 4));

            if (values) {
                console.log(values);
                for (let name of CalibrationEditor.cameraNames) {
                    if (!values[name]) {
                        continue;
                    }
                    // let calibCallbackData = { cameraName: this.cameraName, cam_cc:
                    // new Array(), cam_fc: new Array(), cam_ext: new Array() };
                    let val = values[name];
                    let prettyPrintData = "";
                    let savedConfig = false;
                    console.log(val);
                    if (val.doIntrinsic == true && val.cam_cc &&
                        val.cam_cc.length == 2 && val.cam_fc &&
                        val.cam_fc.length == 2) {
                        prettyPrintData += "Intrinsics \n fc: ";
                        prettyPrintData += JSON.stringify(val.cam_fc) + "\n cc:";
                        prettyPrintData += JSON.stringify(val.cam_cc) + "\n";
                        if (confirm(
                                'Set ' + val.cameraName + ' camera Intrinsic params?')) {
                            configMan.set(
                                "Brain.Projection", val.cameraName + "_cc", val.cam_cc);
                            configMan.set(
                                "Brain.Projection", val.cameraName + "_fc", val.cam_fc);
                            savedConfig = true;
                        } else {
                            // Do nothing!
                        }
                    } else {
                        console.log("no intrinsic");
                    }

                    if (val.doExtrinsic == true && val.cam_ext &&
                        val.cam_ext.length == 3) {
                        prettyPrintData += "Extrinsics: ";
                        prettyPrintData += JSON.stringify(val.cam_ext) + "\n";
                        if (confirm(
                                'Set ' + val.cameraName + ' Camera Extrinsic params?')) {
                            configMan.set(
                                "Brain.Projection", val.cameraName + "_ext", val.cam_ext);
                            savedConfig = true;
                        } else {
                            // Do nothing!
                        }

                        //  if (confirm('Set ' + val.cameraName + ' Camera Extrinsic
                        //  params?')) {
                        //     configMan.set("Brain.Projection", val.cameraName + "_ext",
                        //     val.cam_ext);
                        //     savedConfig = true;
                        // } else {
                        //     // Do nothing!
                        // }
                    } else {
                        console.log("no extrinsic");
                    }

                    // Persist
                    if (savedConfig && confirm('Persist on Nao?')) {
                        console.log("persist!");
                        configMan.save();
                    }

                    // this.preformattedArea.innerHTML = prettyPrintData;
                    // console.log(prettyPrintData);

                    this.snapshots[val.cameraName] = [];
                }

                if (values.doTorso) {
                    // prettyPrintData += "\nTorso: ";
                    // prettyPrintData += JSON.stringify(values.torsoCalibration) + "\n";
                    if (confirm('Set Torso Calibration?')) {
                        configMan.set(
                            "Brain.Projection", "torsoCalibration",
                            values.torsoCalibration);
                        configMan.save();
                    }
                }
            } else {
                console.log("no cam name");
            }
            calibTools = null;
        }.bind(this));
};

/**
 * Add scripts to the page
 * NOTE: Not working properly.
 */
CalibrationEditor.addScript = function (src) {
    var s = document.createElement('script');
    s.setAttribute('src', src);
    document.body.appendChild(s);
};
CalibrationEditor.download = function (filename, text, type) {
    var element = document.createElement('a');
    element.setAttribute(
        'href', 'data:' + ((type) ? type : 'text/plain') + ';charset=utf-8,' +
        encodeURIComponent(text));
    element.setAttribute('download', filename);

    element.style.display = 'none';
    document.body.appendChild(element);

    element.click();

    document.body.removeChild(element);
}

module.exports = CalibrationEditor;
