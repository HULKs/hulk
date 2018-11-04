var _ = require('underscore');
var util = require('./ui').util;
var DebugDisplay = require('./debugDisplay');
var _ = require('underscore');
var Map = function () {
    this.init.apply(this, arguments);
};

util.inherits(Map, DebugDisplay);

Map.defaultConfig = {
    title: 'Map',
    width: 1040,
    showLines: '',
    showParticleLines: '',
    showRasterization: '',
    showGoalPosts: '',
    showPathplannning: '',
    showObstacles: '',
    showBallSearchProbabilityMap: '',
    showProbMapIndices: '',
    showProbMapAge: '',
    showSearchPose: '',
    showPotentialSearchPoses: ''
};

var splField = {
    name: "SPL (full size)",
    length: 9,
    width: 6,
    border: 0.7,
    lineWidth: 0.05,
    penaltyMarkSize: 0.1,
    penaltyMarkDistance: 1.3,
    penaltyAreaLength: 0.6,
    penaltyAreaWidth: 2.2,
    centerCircleDiameter: 1.5
};

var labField = {
    name: "small lab",
    length: 4.5,
    width: 3,
    border: 0.7,
    lineWidth: 0.05,
    penaltyMarkSize: 0.1,
    penaltyMarkDistance: 1.0,
    penaltyAreaLength: 0.6,
    penaltyAreaWidth: 1.3,
    centerCircleDiameter: 1.0
};

var smdField = {
    name: "SMD",
    length: 7.5,
    width: 5,
    border: 0.4,
    lineWidth: 0.05,
    penaltyMarkSize: 0.1,
    penaltyMarkDistance: 1.3,
    penaltyAreaLength: 0.6,
    penaltyAreaWidth: 2.2,
    centerCircleDiameter: 1.25
};

var field = splField; //splField; //labField;
var fields = [splField, smdField, labField];


function FieldSelector() {
    this.init.apply(this, arguments);
}

FieldSelector.prototype.init = function (container, preSelect) {
    this.fieldSelection = document.createElement('select');

    var fieldOption = document.createElement('option');
    fields.map(fieldType => {
        var element = document.createElement('option');
        element.value = fields.indexOf(fieldType);
        element.appendChild(document.createTextNode(fieldType.name));
        this.fieldSelection.appendChild(element);
    });

    container.appendChild(this.fieldSelection);
};


FieldSelector.prototype.onChange = function (handler) {
    this.fieldSelection.addEventListener('change', handler, false);
};

FieldSelector.prototype.value = function () {
    return fields[this.fieldSelection.value];
};

Map.expectedKeys = [['object']];

// inherited from DebugDisplay
Map.prototype.init = function (config) {

    //super class
    DebugDisplay.prototype.init.call(this, config);
    //event handlers
    this.onUpdate = _.bind(this.onUpdate, this);

    //config
    this.config = _.defaults(config || {}, Map.defaultConfig);

    //graphics
    this.selector = new FieldSelector(this.wrapper)
    this.selector.onChange(_.bind(this.updateFieldSize, this));

    this.canvas = document.createElement('canvas');
    this.canvas.width = this.config.width;
    var aspectRatio = (2 * field.border + field.width) / (2 * field.border + field.length);
    this.canvas.height = aspectRatio * this.config.width;
    this.wrapper.appendChild(this.canvas);

    this.ctx = this.canvas.getContext('2d');

    this.buf = new Array(this.config.bufferSize);

    this.subscribe(config.keys, config.mappingFct);
    requestAnimationFrame(_.bind(this.paint, this));
};

Map.prototype.updateFieldSize = function () {
    field = this.selector.value();
};

Map.prototype.onUpdate = function (objects) {
    this.particles = objects.particles;
    this.pose = objects.pose;
    this.ball = objects.ball;
    this.teamBall = objects.teamBall;
    this.lineData = objects.lineData;
    this.teamPlayers = objects.teamPlayers;
    this.motionPlanner = objects.motionPlanner;
    this.jointSensorData = objects.jointSensorData;
    if (this.jointSensorData) {
        this.headYaw = this.jointSensorData.angles[0];
    }
    else {
        this.headYaw = 0.0;
    }

    // Ball search prob map
    this.ballSearchProbabilityMap = objects.ballSearchProbabilityMap;
    this.ballSearchPose = objects.ballSearchPose;
    this.potentialSearchPoses = objects.potentialSearchPoses;
    this.probScaleFunction = (objects.probScaleFunction) ? objects.probScaleFunction : (n) => n;
};
// map specific
Map.prototype.paintField = function () {

    this.ctx.fillStyle = '#090';
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

    // Line size and color:
    this.ctx.strokeStyle = '#fff';
    this.ctx.imageSmoothingEnabled = false;
    this.ctx.lineWidth = this.pixelField.lineWidth;

    this.ctx.beginPath();
    //field border
    this.ctx.rect(this.pixelField.border, this.pixelField.border, this.pixelField.length, this.pixelField.width);
    //penalty area left
    this.ctx.rect(this.pixelField.border, (this.pixelField.border + (this.pixelField.width / 2) - (this.pixelField.penaltyAreaWidth / 2)),
        this.pixelField.penaltyAreaLength, this.pixelField.penaltyAreaWidth);
    //penalty area right
    this.ctx.rect(this.pixelField.border + this.pixelField.length - this.pixelField.penaltyAreaLength, (this.pixelField.border + (this.pixelField.width / 2) - (this.pixelField.penaltyAreaWidth / 2)),
        this.pixelField.penaltyAreaLength, this.pixelField.penaltyAreaWidth);

    //center line
    this.ctx.moveTo(this.pixelField.border + this.pixelField.length / 2, this.pixelField.border);
    this.ctx.lineTo(this.pixelField.border + this.pixelField.length / 2, this.pixelField.border + this.pixelField.width);
    this.ctx.stroke();

    //center circle
    this.ctx.beginPath();
    this.ctx.arc(this.pixelField.border + this.pixelField.length / 2, this.pixelField.border + this.pixelField.width / 2,
        this.pixelField.centerCircleDiameter / 2, 0, 2 * Math.PI);
    this.ctx.stroke();

    //penalty marks
    this.ctx.beginPath();
    this.ctx.fillStyle = '#fff';
    this.ctx.arc(this.pixelField.border + this.pixelField.penaltyMarkDistance, this.pixelField.border + this.pixelField.width / 2,
        this.pixelField.penaltyMarkSize / 2, 0, 2 * Math.PI);
    this.ctx.fill();

    this.ctx.beginPath();
    this.ctx.fillStyle = '#fff';
    this.ctx.arc(this.pixelField.border + this.pixelField.length - this.pixelField.penaltyMarkDistance, this.pixelField.border + this.pixelField.width / 2,
        this.pixelField.penaltyMarkSize / 2, 0, 2 * Math.PI);
    this.ctx.fill();

    //kick off point
    this.ctx.beginPath();
    this.ctx.fillStyle = '#fff';
    this.ctx.arc(this.pixelField.border + this.pixelField.length / 2, this.pixelField.border + this.pixelField.width / 2,
        this.pixelField.penaltyMarkSize / 2, 0, 2 * Math.PI);
    this.ctx.fill();
};

// Ball search prob map
Map.prototype.drawBallSearchProbMap = function() {
    if(!this.ballSearchProbabilityMap) return;

    //TODO: Check how these are associated with height and width
    const nCols = this.ballSearchProbabilityMap.length;
    const nRows = this.ballSearchProbabilityMap[0].length;

    // Calculate width and height of the probTile
    var h = this.pixelField.width / (nRows - 2);
    var w = this.pixelField.length / (nCols - 2);
    var borderOffset = this.pixelField.border;

    for (var j = 0; j < nCols; ++j) {
        for (var i = 0; i < nRows; ++i) {
            const tilePosX = borderOffset + (j - 1) * w;
            const tilePosY = borderOffset + (nRows - 2 - i) * h;
            const tileAgePosX = tilePosX + w / 4;
            const tileAgePosY = tilePosY + h / 3;
            const textPosX = tilePosX + w / 2;
            const textPosY = tilePosY + 2 * h / 3;
            this.ctx.fillStyle = this.rgba(0, 1, 1, this.probScaleFunction(this.ballSearchProbabilityMap[j][i].probability));
            this.ctx.fillRect(tilePosX, tilePosY, w, h);
            if (this.ballSearchProbabilityMap[j][i].age < 5000) { // TODO: 1000 = minAgeToSearch
                this.ctx.fillStyle = this.rgba(0, 1, 0, this.ballSearchProbabilityMap[j][i].age  / 5000); // TODO: 1000 = minAgeToSearch
            } else if (this.ballSearchProbabilityMap[j][i].age < 10000) {
                this.ctx.fillStyle = this.rgba(1, 1, 0, 1);
            } else {
                this.ctx.fillStyle = this.rgba(1, 0, 0, 1);
            }
            this.ctx.fillRect(tileAgePosX, tileAgePosY, w / 2, h / 8);
            this.ctx.fillStyle = 'black';
            this.ctx.textAlign = 'center';
            this.ctx.font = "8px Arial";
            if (this.config.showProbMapIndices) {
                // TODO: Also display timestamps here
                // Show tile indices for debugging:
                this.ctx.fillText('(' + i + ', ' + j + ')', textPosX, textPosY);
            } else if (this.config.showProbMapAge) {
                this.ctx.fillText(Math.round(this.ballSearchProbabilityMap[j][i].probability * 10000) / 10000, textPosX, textPosY);
            }
        }
    }

    if (this.config.showSearchPose)
    {
        const searchPoseStyle = this.rgba(1, 0, 1, 0.5);
        this.drawPose(this.ballSearchPose, searchPoseStyle);
    }
    if (this.config.showPotentialSearchPoses) {
        const nSearchPoses = this.potentialSearchPoses.length;
        const searchPoseStyle = this.rgba(1, 0, 0, 0.2);

        for (var i = 0; i < nSearchPoses; ++i) {
            if  (typeof(this.potentialSearchPoses[i])!=='undefined') {
                this.drawPosition(this.potentialSearchPoses[i].position, searchPoseStyle, "?");
            }
            else {
                console.log("UNDEFINED!");
            }
        }
    }
}

/**
 * returns a vector of the form [x, y, alpha] from a pose [[x, y], alpha]
 * @param pose
 */
Map.prototype.getVectorFromPose = function (pose) {
    if (!pose) return [0, 0, 0];

    return pose[0].concat(pose[1])
};

Map.prototype.getPixelCoordinates = function (vector) {
    if (!vector) return [0, 0, 0];

    var m = this.scaling;

    var x = vector[0];
    var y = vector[1];
    var alpha = vector[2];

    mapX = x + field.border + field.length / 2;
    mapY = -y + field.border + field.width / 2;

    return [mapX * m, mapY * m, -alpha];
};

Map.prototype.getAbsoluteCoordinates = function (robotCoordinates) {
    if (!this.pose) return undefined;
    var pos = this.pose;

    var x = Math.cos(pos[1]) * robotCoordinates[0] - Math.sin(pos[1]) * robotCoordinates[1] + pos[0][0];
    var y = Math.sin(pos[1]) * robotCoordinates[0] + Math.cos(pos[1]) * robotCoordinates[1] + pos[0][1];

    return [x, y];
};

Map.prototype.getAbsolutePose = function (pose) {
    if (!this.pose) return undefined;
    var absPos = this.getAbsoluteCoordinates(pose[0]);
    return [absPos, this.pose[1] + pose[1]];
};

Map.prototype.rgba = function (r, g, b, a) {
    var rgba = [r * 255, g * 255, b * 255, a];
    return 'rgba(' + rgba.join(', ') + ')';
};

Map.prototype.drawTriangle = function (CornersAsVector, style, opacity) {
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
    this.ctx.globalAlpha = 1.0;
};

Map.prototype.drawPose = function (pose, style, annotation) {
    var poseSize = 5;

    this.ctx.beginPath();

    this.ctx.strokeStyle = '#000';
    this.ctx.lineWidth = 1;

    var pPos = this.getPixelCoordinates(this.getVectorFromPose(pose));
    this.ctx.arc(pPos[0], pPos[1], poseSize, 0, 2 * Math.PI);
    this.ctx.stroke();

    this.ctx.fillStyle = style;
    this.ctx.fill();


    this.ctx.beginPath();
    this.ctx.moveTo(pPos[0], pPos[1]);
    this.ctx.lineTo(pPos[0] + 2 * poseSize * Math.cos(pPos[2]), pPos[1] + 2 * poseSize * Math.sin(pPos[2]));
    this.ctx.stroke();

    // Draw annotation for pose:
    if (annotation) {
        this.ctx.font = "20px Arial";
        this.ctx.fillText(annotation, pPos[0] + poseSize, pPos[1] - poseSize);
    }

};

Map.prototype.drawFov = function (color, opacity) {
    // visualize the field of view fov:
    // the lateral opening angle of the camera is about 40° to each side:
    var maxDistance = 3;
    var TO_RAD = Math.PI / 180.0;
    var cameraOpeningAngle = 60.97 * TO_RAD;
    var cameraPosition = [0.0, -0.0];
    var leftCorner = [Math.cos(this.headYaw + cameraOpeningAngle * 0.5) * maxDistance, Math.sin(this.headYaw + cameraOpeningAngle * 0.5) * maxDistance];
    var rightCorner = [Math.cos(this.headYaw - cameraOpeningAngle * 0.5) * maxDistance, Math.sin(this.headYaw - cameraOpeningAngle * 0.5) * maxDistance];
    // to absolute pixel coordinates
    cameraPosition = this.getPixelCoordinates(this.getAbsoluteCoordinates(cameraPosition));
    leftCorner = this.getPixelCoordinates(this.getAbsoluteCoordinates(leftCorner));
    rightCorner = this.getPixelCoordinates(this.getAbsoluteCoordinates(rightCorner));
    // draw fov cone
    this.drawTriangle([cameraPosition[0], cameraPosition[1],
            leftCorner[0], leftCorner[1],
            rightCorner[0], rightCorner[1]]
        , color, opacity);
};

Map.prototype.drawMotionPlannerTranslation = function (style, scale) {
    var transX = this.motionPlanner.translation[0] * scale;
    var transY = this.motionPlanner.translation[1] * scale;

    var startCoordPixel = this.getPixelCoordinates(this.getVectorFromPose(this.pose));
    var endCoordPixel = this.getPixelCoordinates(this.getAbsoluteCoordinates([transX, transY]));

    this.drawArrow(startCoordPixel[0], startCoordPixel[1], endCoordPixel[0], endCoordPixel[1], style, 5, 2);
};

Map.prototype.drawPosition = function (position, style, annotation) {
    var positionSize = 5;

    this.ctx.beginPath();
    this.ctx.strokeStyle = '#000';
    this.ctx.lineWidth = 1;

    // Inner circle
    var pPos = this.getPixelCoordinates(this.getVectorFromPose([position, 0]));
    //console.log(pPos);
    this.ctx.arc(pPos[0], pPos[1], positionSize, 0, 2 * Math.PI);
    this.ctx.stroke();
    // Outer circle
    this.ctx.arc(pPos[0], pPos[1], positionSize * 2, 0, 2 * Math.PI);
    this.ctx.strokeStyle = style;
    this.ctx.stroke();
    // Cross
    this.ctx.beginPath();
    this.ctx.moveTo(pPos[0] - 2 * positionSize, pPos[1]);
    this.ctx.lineTo(pPos[0] + 2 * positionSize, pPos[1]);
    this.ctx.stroke();
    this.ctx.beginPath();
    this.ctx.moveTo(pPos[0], pPos[1] - 2 * positionSize);
    this.ctx.lineTo(pPos[0], pPos[1] + 2 * positionSize);
    this.ctx.stroke();

    // Draw annotation for position:
    if (annotation) {
        this.ctx.font = "20px Arial";
        this.ctx.fillStyle = style;
        this.ctx.fillText(annotation, pPos[0] + positionSize, pPos[1] - positionSize);
    }
};

// go through all obstacles and draws a circle for each one
Map.prototype.drawObstacles = function (color) {
    if (!this.motionPlanner || !this.config.showObstacles) return;

    // Set line style for dashed obstacle circles
    this.ctx.strokeStyle = color;
    this.ctx.setLineDash([10, 5]);
    this.ctx.lineWidth = 2;

    // Iterate through all obstacles and radii and use zip() to create pairs called
    // obstacleElement to conveniently draw obstacles with their corresponding radii.
    for (const obstacleElement of _.zip(this.motionPlanner.obstacles, this.motionPlanner.avoidanceRadii)) {
        this.ctx.beginPath();
        // Obstacle circle
        const obstacle = obstacleElement[0];
        const avoidanceRadius = obstacleElement[1];
        const absCoords = this.getAbsoluteCoordinates(obstacle.position);
        const pixCoords = this.getPixelCoordinates([absCoords[0], absCoords[1], 0]);
        const radius = avoidanceRadius * this.scaling;
        this.ctx.arc(pixCoords[0], pixCoords[1], radius, 0, 2 * Math.PI);
        this.ctx.stroke();
    }

    // Reset line style to solid line
    this.ctx.setLineDash([0]);
};

Map.prototype.drawParticle = function (particle) {
    // Draw the basic pose
    const style = this.rgba(1, 0, 0, particle.weight);
    this.drawPose(particle.pose, style);

    //lines, the particle sees:
    if (this.config.showParticleLines) {
        this.ctx.beginPath();
        this.ctx.strokeStyle = '#0ff';
        this.ctx.lineWidth = 1;

        _.each(particle.lines, function (line) {
            var v = this.getPixelCoordinates(line[0]);
            var w = this.getPixelCoordinates(line[1]);

            this.ctx.moveTo(v[0], v[1]);
            this.ctx.lineTo(w[0], w[1]);
        }, this);
        this.ctx.stroke();
    }

    if (this.config.showRasterization) {
        this.drawPointCloud(particle.rasterization, '#0ff', 2);
    }

    if (this.config.showGoalPosts) {
        this.drawPointCloud(particle.goalPosts, '#ff0', 4);
    }
};

Map.prototype.drawPointCloud = function (points, style, radius) {
    this.ctx.strokeStyle = style;

    _.each(points, function (point) {
        var p = this.getPixelCoordinates(point);
        this.ctx.beginPath();
        this.ctx.arc(p[0], p[1], radius, 0, 2 * Math.PI);
        this.ctx.stroke();
    }, this);

};

Map.prototype.drawPlayerPosition = function () {
    if (!this.pose) return;
    if (this.config.showPathplannning) {
        this.drawTarget("#f9f688");
        this.drawMotionPlannerTranslation("#ff0", 0.33); // Scale the translation arrow down to make it less distracting
    }

    this.drawPose(this.pose, '#00f');
    this.drawFov('red', 0.1);
};

Map.prototype.drawBall = function (absoluteBallPosition, fillStyle) {
    if (!absoluteBallPosition) return;

    this.ctx.beginPath();
    this.ctx.fillStyle = fillStyle;
    this.ctx.strokeStyle = fillStyle;

    ballRadius = 0.05 * this.scaling;

    var ball = this.getPixelCoordinates(absoluteBallPosition);
    this.ctx.arc(ball[0], ball[1], ballRadius, 0, 2 * Math.PI);

    this.ctx.fill();
};

Map.prototype.drawLines = function (style) {
    if (!this.lineData || !this.config.showLines) return;
    this.ctx.strokeStyle = style;
    this.ctx.lineWidth = 3;
    this.ctx.beginPath();
    _.each(this.lineData.edges, function (edge) {

        var startPoint = this.getPixelCoordinates(this.getAbsoluteCoordinates(this.lineData.vertices[edge[0]]));
        var endPoint = this.getPixelCoordinates(this.getAbsoluteCoordinates(this.lineData.vertices[edge[1]]));

        this.ctx.moveTo(startPoint[0], startPoint[1]);
        this.ctx.lineTo(endPoint[0], endPoint[1]);
    }, this);

    this.ctx.stroke();
};

Map.prototype.getStyleForRole = function (role) {
    switch (role) {
        case 1 : // Keeper
            return 'white';
        case 2 : // Defender
            return 'lightgreen';
        case 3 : // Supporter
            return 'pink';
        case 4 : // Striker
            return 'red';
        case 5 : // Bishop
            return 'yellow';
        default :
            return 'grey';
    }
};

Map.prototype.drawTeamPlayers = function () {
    if (!this.teamPlayers) {
        //console.log ("No TeamPlayers data send");
        return;
    }
    if (this.teamPlayers.players.length == 0) {
        //console.log("No player in TeamPlayers.player");
        return;
    }
    //console.log("Team Mates: " + this.teamPlayers.players.length);
    // The TeamPlayers data provides an array of players
    for (const player of this.teamPlayers.players) {
        playerColor = this.getStyleForRole(player.currentlyPerfomingRole);
        // Draw the players target pose
        this.drawPosition(player.target, playerColor, player.playerNumber);
        // Draw the players position:
        this.drawPose(player.pose, playerColor, player.playerNumber);
    }
}

Map.prototype.paint = function () {
    this.scaling = this.canvas.width / (2 * field.border + field.length);

    this.pixelField = {};
    for (var key in field) {
        this.pixelField[key] = field[key] * this.scaling;
    }
    //Draw field
    this.paintField();
    _.each(this.particles, this.drawParticle, this);

    // Draw own ball
    if (this.ball && this.ball.found) {
        this.drawBall(this.getAbsoluteCoordinates(this.ball.position), '#000');
        this.drawBall(this.getAbsoluteCoordinates(this.ball.destination), '#f0f');
    }
    // Draw team ball
    if (this.teamBall && this.teamBall.seen) {
        teamBallStateColor = this.teamBall.found ? '#98FB98' : '#FF0000';
        this.drawBall(this.teamBall.position, teamBallStateColor);
    }
    // Do all the stuff to visualize the team players
    this.drawTeamPlayers();
    // Draw the position of this robot
    this.drawPlayerPosition();
    // Draw the lines, this robot sees
    this.drawLines('#f00');
    // Draw the obstacle output from motionplanning
    this.drawObstacles('#ff6400');

    // Draw prob map
    if (this.config.showBallSearchProbabilityMap) {
        this.drawBallSearchProbMap();
    }

    requestAnimationFrame(_.bind(this.paint, this));
};


/***
 * Draws target pose and a connection to current pose
 * @param color of target and connection
 */
Map.prototype.drawTarget = function (color) {
    var absTargetPose = this.getAbsolutePose(this.motionPlanner.walkTarget);
    this.drawPose(absTargetPose, color);

    //draw connection from robot to target
    var ownPose = this.getPixelCoordinates(this.getVectorFromPose(this.pose));
    var targetPose = this.getPixelCoordinates(this.getVectorFromPose(absTargetPose));
    this.ctx.beginPath();
    this.ctx.moveTo(ownPose[0], ownPose[1]);
    this.ctx.lineTo(targetPose[0], targetPose[1]);
    this.ctx.lineWidth = 1;
    this.ctx.strokeStyle = color;
    this.ctx.setLineDash([3, 3]);
    this.ctx.stroke();
    // Reset line style to solid line
    this.ctx.setLineDash([0]);
};


/***
 * Draws a arrow from PIXEL COORDINATES - [fromX, fromY] to [toX, toY] with optional headLength
 * @param fromX
 * @param fromY
 * @param toX
 * @param toY
 * @param style
 * @param headLength
 * @param width
 */
Map.prototype.drawArrow = function (fromX, fromY, toX, toY, style, headLength, width) {
    ///Default values
    if (typeof headLength === 'undefined') {
        headLength = 5;
    }
    if (!width) {
        width = 1;
    }
    var dy = toY - fromY;
    var dx = toX - fromX;
    var angle = Math.atan2(dy, dx);

    this.ctx.beginPath();
    this.ctx.moveTo(fromX, fromY);
    this.ctx.lineTo(toX, toY);
    this.ctx.lineTo(toX - headLength * Math.cos(angle - Math.PI / 6), toY - headLength * Math.sin(angle - Math.PI / 6));
    this.ctx.moveTo(toX, toY);
    this.ctx.lineTo(toX - headLength * Math.cos(angle + Math.PI / 6), toY - headLength * Math.sin(angle + Math.PI / 6));

    this.ctx.lineWidth = width;
    this.ctx.strokeStyle = style;
    this.ctx.stroke();
};

module.exports = Map;
