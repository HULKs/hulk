var util = require('./ui').util;
var DebugDisplay = require('./debugDisplay');
var _ = require('underscore');

var RelativeMap = function () {
    this.init.apply(this, arguments);
};

util.inherits(RelativeMap, DebugDisplay);

RelativeMap.defaultConfig = {
    title: 'RelativeMap',
    size: 500,
    gridSize: 0.5,
    pixelsPerMeter: 100
};

RelativeMap.expectedKeys = [['object']];

// inherited from DebugDisplay
RelativeMap.prototype.init = function (config) {
    //super class
    DebugDisplay.prototype.init.call(this, config);
    //event handlers
    this.onUpdate = _.bind(this.onUpdate, this);

    // config
    this.config = _.defaults(config || {}, RelativeMap.defaultConfig);
    this.drawGridCoordinateSystem = false;

    // add toggle buttons
    // coordiante system switch
    if (!this.btnCoordinateSystem) {
        this.btnCoordinateSystem = document.createElement("div");
        this.btnCoordinateSystem.addEventListener("click", _.bind(this.switchCoordinateSystem, this), false);
        this.head.appendChild(this.btnCoordinateSystem);
    }
    // sonar view
    if (!this.btnSonarView) {
        this.btnSonarView = document.createElement("div");
        this.btnSonarView.addEventListener("click", _.bind(this.switchSonarView, this), false);
        this.head.appendChild(this.btnSonarView);
    }

    // set button settings
    this.btnCoordinateSystem.isToggled = false
    this.btnCoordinateSystem.style.width = "135px";
    this.btnCoordinateSystem.align = "center";
    this.btnCoordinateSystem.className = "btn space";
    this.btnCoordinateSystem.innerHTML = "Switch to cartesian";
    this.btnCoordinateSystem.title = "Switch to cartesian coordinate system.";

    this.btnSonarView.isToggled = false
    this.btnSonarView.style.width = "135px";
    this.btnSonarView.align = "center";
    this.btnSonarView.className = "btn space";
    this.btnSonarView.innerHTML = "Sonar View : On/Off";
    this.btnSonarView.title = "";

    //graphics
    this.canvas = document.createElement("canvas");
    this.canvas.width = this.config.size;
    this.canvas.height = this.config.size;
    this.wrapper.appendChild(this.canvas);

    this.ctx = this.canvas.getContext('2d');

    //this.ballState.position = [1, 0];

    this.subscribe(config.keys, config.mappingFct);
    requestAnimationFrame(_.bind(this.paint, this));
};

RelativeMap.prototype.onUpdate = function (input) {
    this.ballState = input['ballState'];
    this.lineData = input['lineData'];
    this.sonarRight = input['sonarRight'];
    this.sonarLeft = input['sonarLeft'];
};

RelativeMap.prototype.paint = function () {
    this.ctx.fillStyle = '#090';
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

    if (this.btnCoordinateSystem.isToggled) {
        this.drawGrid('#fff');
    } else {
        this.drawPolarGrid('#fff');
    }
    if (this.ballState && this.ballState.found) {
        this.drawBall('#ff000d');
    }
    if (this.lineData) {
        this.drawLines('#ff000d');
    }
    if (this.btnSonarView.isToggled) {
        this.drawSonarView("#f0f", "'#FECDFF");
    }

    requestAnimationFrame(_.bind(this.paint, this));
};

RelativeMap.prototype.drawGrid = function (style) {

    var gridSize = (+this.config.gridSize);

    var m = +this.config.pixelsPerMeter;

    this.ctx.strokeStyle = style;
    this.ctx.font = "10px Arial";
    var lengthOfMarks = 10;
    var mainAxisWidth = 2;
    var subAxisWidth = 1;

    // X axis
    this.ctx.beginPath();
    this.ctx.lineWidth = mainAxisWidth;
    this.ctx.moveTo(this.canvas.width / 2, 0);
    this.ctx.lineTo(this.canvas.width / 2, this.canvas.height);
    this.ctx.strokeText("x", this.canvas.width / 2 + lengthOfMarks / 2, 10);
    this.ctx.stroke();
    // marks & subAxis
    for (var n = gridSize * m; n < this.canvas.height / 2; n += gridSize * m) {
        this.ctx.beginPath();
        this.ctx.lineWidth = subAxisWidth;
        this.ctx.moveTo(0, this.canvas.height / 2 + n);
        this.ctx.lineTo(this.canvas.width, this.canvas.height / 2 + n);
        this.ctx.moveTo(0, this.canvas.height / 2 - n);
        this.ctx.lineTo(this.canvas.width, this.canvas.height / 2 - n);
        this.ctx.stroke();
    }

    // Y axis
    this.ctx.beginPath();
    this.ctx.lineWidth = mainAxisWidth;
    this.ctx.moveTo(0, this.canvas.height / 2);
    this.ctx.lineTo(this.canvas.width, this.canvas.height / 2);
    this.ctx.strokeText("y", 10, this.canvas.height / 2 - lengthOfMarks);
    this.ctx.stroke();
    // marks & subAxis
    for (var n = gridSize * m; n < this.canvas.width / 2; n += gridSize * m) {
        this.ctx.beginPath();
        this.ctx.lineWidth = subAxisWidth;
        this.ctx.moveTo(this.canvas.width / 2 + n, 0);
        this.ctx.lineTo(this.canvas.width / 2 + n, this.canvas.height);
        this.ctx.moveTo(this.canvas.width / 2 - n, 0);
        this.ctx.lineTo(this.canvas.width / 2 - n, this.canvas.height);
        this.ctx.stroke();
    }
    //draw legend
    this.ctx.strokeText("box = 1x1m", this.canvas.width * 0.9, this.canvas.height - 1); // position
};

RelativeMap.prototype.drawPolarGrid = function (style) {
    this.ctx.strokeStyle = style;

    var gridSize = (+this.config.gridSize);
    var m = +this.config.pixelsPerMeter;
    this.ctx.lineWidth = 1;
    var centerPoint = [this.canvas.height / 2, this.canvas.width / 2];

    // circles
    for (var r = gridSize * m; r < this.canvas.height; r += gridSize * m) {
        this.ctx.beginPath();
        this.ctx.arc(centerPoint[0], centerPoint[1], r, 0, 2 * Math.PI);
        this.ctx.stroke();
    }
    // lines
    for (var i = 0; i < 16; i++) {
        this.ctx.beginPath();
        this.ctx.moveTo(this.canvas.height / 2, this.canvas.height / 2);

        var p = this.polar2cartesian([100, Math.PI / 4 * i]);

        this.ctx.lineTo(m * p[0] + centerPoint[0], m * p[1] + centerPoint[1]);
        this.ctx.stroke();
    }
};

RelativeMap.prototype.drawBall = function (style) {
    var m = +this.config.pixelsPerMeter;

    var ballPosition = this.getPixelCoordinates(this.ballState.position);

    this.ctx.fillStyle = style;
    this.ctx.beginPath();
    this.ctx.arc(ballPosition[0], ballPosition[1], 0.05 * m, 0, 2 * Math.PI);
    this.ctx.fill();
    this.ctx.strokeStyle = "#000";
    this.ctx.stroke();
};

RelativeMap.prototype.drawLines = function (style) {
    var m = +this.config.pixelsPerMeter;

    this.ctx.strokeStyle = style;
    this.ctx.lineWidth = 0.05 * m;
    this.ctx.beginPath();
    _.each(this.lineData.edges, function (edge) {

        var startPoint = this.getPixelCoordinates(this.lineData.vertices[edge[0]]);
        var endPoint = this.getPixelCoordinates(this.lineData.vertices[edge[1]]);

        this.ctx.moveTo(startPoint[0], startPoint[1]);
        this.ctx.lineTo(endPoint[0], endPoint[1]);
    }, this);

    this.ctx.stroke();
};
/**
 * Converts relative to pixel coordinates
 * @param vector of [*,*,*]
 * @returns {[*,*,*]}
 */
RelativeMap.prototype.getPixelCoordinates = function (vector) {
    var m = +this.config.pixelsPerMeter;

    var x = vector[0];
    var y = vector[1];
    var alpha = vector[2] || 0;

    return [(this.canvas.width / 2) - (y * m), (this.canvas.height / 2) - (x * m), alpha];
};


RelativeMap.prototype.cartesian2polar = function (cartesian) {
    var x = cartesian[0];
    var y = cartesian[1];

    var r = Math.sqrt(x * x + y * y);
    var phi = Math.atan2(x, y);

    return [r, phi];
};

RelativeMap.prototype.polar2cartesian = function (polar) {
    var r = polar[0];
    var phi = polar[1];

    var x = r * Math.cos(phi);
    var y = r * Math.sin(phi);

    return [x, y];
};

RelativeMap.prototype.drawSonarView = function (style) {

    var maxDistance = 1; //max sight distance
    var opacity = 0.1; // opacity of view;

    // right sensor
    var posSensor = [0.0477, -0.0416]; // Position of right receiver relative to robot - NaoV5.
    var leftCorner = [Math.cos(0.0872665) * maxDistance, Math.sin(0.0872665) * maxDistance]; // angle values (-25° + 30°) - NaoV5.
    var rightCorner = [Math.cos(-0.959931) * maxDistance, Math.sin(-0.959931) * maxDistance]; // angle values (-25° - 30°) - NaoV5.
    // to pixel coordinates
    posSensor = this.getPixelCoordinates(posSensor);
    leftCorner = this.getPixelCoordinates(leftCorner);
    rightCorner = this.getPixelCoordinates(rightCorner);
    // draw right cone
    this.drawTriangle([posSensor[0], posSensor[1],
            leftCorner[0], leftCorner[1],
            rightCorner[0], rightCorner[1]]
        , style, opacity);

    // left sensor
    posSensor = [0.0477, +0.0416]; // Position of right receiver relative to robot - NaoV5.
    leftCorner = [Math.cos(0.959931) * maxDistance, Math.sin(0.959931) * maxDistance]; // angle values (+25° + 30°) - NaoV5.
    rightCorner = [Math.cos(-0.0872665) * maxDistance, Math.sin(-0.0872665) * maxDistance]; // angle values (+25° - 30°) - NaoV5.
    // to pixel coordinates
    posSensor = this.getPixelCoordinates(posSensor);
    leftCorner = this.getPixelCoordinates(leftCorner);
    rightCorner = this.getPixelCoordinates(rightCorner);
    //draw cone
    this.drawTriangle([posSensor[0], posSensor[1],
            leftCorner[0], leftCorner[1],
            rightCorner[0], rightCorner[1]]
        , style, opacity);

    // Measurement indicator
    //right
    maxDistance = this.sonarRight; //max sight distance
    if (maxDistance > 1) {
        maxDistance = 1;
    }
    posSensor = [0.0477, -0.0416]; // Position of right receiver relative to robot - NaoV5.
    leftCorner = [Math.cos(0.0872665) * maxDistance, Math.sin(0.0872665) * maxDistance]; // angle values (-25° + 30°) - NaoV5.
    rightCorner = [Math.cos(-0.959931) * maxDistance, Math.sin(-0.959931) * maxDistance]; // angle values (-25° - 30°) - NaoV5.
    // to pixel coordinates
    posSensor = this.getPixelCoordinates(posSensor);
    leftCorner = this.getPixelCoordinates(leftCorner);
    rightCorner = this.getPixelCoordinates(rightCorner);
    // draw right cone
    this.drawTriangle([leftCorner[0], leftCorner[1],
            leftCorner[0], leftCorner[1],
            rightCorner[0], rightCorner[1]]
        , style, 1);

    //left
    maxDistance = this.sonarLeft; //max sight distance
    if (maxDistance > 1) {
        maxDistance = 1;
    }
    posSensor = [0.0477, +0.0416]; // Position of right receiver relative to robot - NaoV5.
    leftCorner = [Math.cos(0.959931) * maxDistance, Math.sin(0.959931) * maxDistance]; // angle values (+25° + 30°) - NaoV5.
    rightCorner = [Math.cos(-0.0872665) * maxDistance, Math.sin(-0.0872665) * maxDistance]; // angle values (+25° - 30°) - NaoV5.
    // to pixel coordinates
    posSensor = this.getPixelCoordinates(posSensor);
    leftCorner = this.getPixelCoordinates(leftCorner);
    rightCorner = this.getPixelCoordinates(rightCorner);
    // draw right cone
    this.drawTriangle([leftCorner[0], leftCorner[1],
            leftCorner[0], leftCorner[1],
            rightCorner[0], rightCorner[1]]
        , style, 1);
};
/**
 * Draws triangle of pixel coordinates
 * @param CornersAsVector - [x1, y1, x2, y2, x3, y3]
 * @param style
 * @param opacity
 */
RelativeMap.prototype.drawTriangle = function (CornersAsVector, style, opacity) {
    var defaultOpacity = this.ctx.globalAlpha; // Need to be held for restoring after drawing.
    var m = +this.config.pixelsPerMeter;

    this.ctx.globalAlpha = opacity  // Set opacity
    this.ctx.strokeStyle = style;

    this.ctx.beginPath();
    this.ctx.moveTo(CornersAsVector[0], CornersAsVector[1]);
    this.ctx.lineTo(CornersAsVector[2], CornersAsVector[3]);
    this.ctx.lineTo(CornersAsVector[4], CornersAsVector[5]);
    this.ctx.fillStyle = style;
    this.ctx.fill();
    this.ctx.closePath();
    this.ctx.stroke();

    this.ctx.globalAlpha = defaultOpacity  // Set opacity back to normal.
}

RelativeMap.prototype.switchCoordinateSystem = function () {
    if (this.btnCoordinateSystem.isToggled === true) {
        this.btnCoordinateSystem.innerHTML = "Switch to cartesian";
        this.btnCoordinateSystem.title = "Switch to cartesian coordinate system.";
    } else {
        this.btnCoordinateSystem.innerHTML = "Switch to polar";
        this.btnCoordinateSystem.title = "Switch to polar coordinate system.";
    }
    this.btnCoordinateSystem.isToggled = !this.btnCoordinateSystem.isToggled;
};

RelativeMap.prototype.switchSonarView = function () {
    this.btnSonarView.isToggled = !this.btnSonarView.isToggled;
};

module.exports = RelativeMap;
