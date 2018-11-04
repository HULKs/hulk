var util = require('./ui').util;
var DebugDisplay = require('./debugDisplay');
var _ = require('underscore');
var debugMan = require('./connection').debugMan;

/**
 * Created by finn on 07.02.15.
 * Modifed by Patrick on 05.06.2015
 * Added send DataPoints Feature
 */

var CalibrationPoints = function () {
    this.init.apply(this, arguments);
};

util.inherits(CalibrationPoints, DebugDisplay);

CalibrationPoints.defaultConfig = {
    title: '',
    mountPoint: 'Brain.ImageReceiver.top_image',
    key: 'calibrationPoints'
};

CalibrationPoints.expectedKeys = [['image']];

/**
 * Init function of CalibrationPoints class
 * Is used to initialize the html features to display the incoming images, close the imagestream
 * and display and send the clicked image point array
 * @param config
 */

CalibrationPoints.prototype.init = function (config) {

    // Calls the init method of parent class
    DebugDisplay.prototype.init.call(this, config);

    this.Points_ = [];      // Saves the clicked points
    this.config = config;


    // Adds a class to the wrapper element
    this.wrapper.classList.add('image');

    // Binds the onUpdate event of this class to the onUpdate method
    this.onUpdate = _.bind(this.onUpdate, this);

    // Creates a canvas element, names it and adds some eventListeners
    this.canvas = document.createElement('canvas');
    this.canvas.width = 640;
    this.canvas.height = 480;
    this.canvas.addEventListener('click', _.bind(this.onClick, this), false);
    this.wrapper.appendChild(this.canvas);

    // Creates a div element to manages the clicked points. It shows the clicked points,
    // adds a button to send the points and allows to delete the points
    this.PointsListSenderButton = document.createElement('input');
    this.PointsListSenderButton.setAttribute('type', 'button');
    this.PointsListSenderButton.setAttribute('value', 'Send Points List');
    this.PointsListSenderButton.addEventListener('click', _.bind(this.sendData, this), false);
    this.panel.appendChild(this.PointsListSenderButton);

    this.pointslist = document.createElement('div');
    this.pointslist.className = 'PointsList';
    this.panel.appendChild(this.pointslist);

    // Stores the draw context of the canvas element
    this.ctx = this.canvas.getContext('2d');

    // Stores the Subscription
    this.subscription = debugMan.subscribeImage(config.keys[0], this.onUpdate);
};

/**
 * OnClick Method of canvas element#
 * It displays the clicked points, adds them to the storage object
 */

CalibrationPoints.prototype.onClick = function (event) {

    //TODO: Offset Error in different Browsers
    /**
     * Has to be tested
     *
     * x = event.clientX + document.body.scrollLeft + document.documentElement.scrollLeft;
     * y = event.clientY + document.body.scrollTop + document.documentElement.scrollTop;
     */

        // Get the Offset form canvas element in browser
    var Offset = findOffset(this.canvas);
    var MouseX_ = 0;       // Saves the actual mouse position in x
    var MouseY_ = 0;       // Saves the actual mouse position in y

    MouseX_ = event.clientX - Offset.x;
    MouseY_ = event.clientY - Offset.y;

    // create dummy point obj for saving reasons and fill with actual mouse coordinats
    var Point = {};
    Point.x = MouseX_;
    Point.y = MouseY_;

    // display points in list below image
    var NewPoint = document.createElement('div');
    NewPoint.className = 'PointListItem';
    NewPoint.appendChild(document.createTextNode(MouseX_ + ' | ' + MouseY_));
    NewPoint.addEventListener('click', _.bind(this.onPointClick, this, Point), false);
    Point.button = NewPoint;
    this.pointslist.appendChild(NewPoint);

    // assign dummy point to points
    this.Points_.push(Point);

    // display all points in image
    this.displayPoints();
};


CalibrationPoints.prototype.onUpdate = function (image) {
    console.log('update image');
    var img = new Image();
    img.src = 'image/' + image + '?' + new Date().getTime();

    img.addEventListener('load', _.bind(function () {
        this.ctx.drawImage(img, 0, 0);
        this.displayPoints();
    }, this));

};

CalibrationPoints.prototype.displayPoints = function () {
    // Display the clicked points in image
    this.ctx.fillStyle = "#FF0000";

    var canvas = this.ctx;
    _.each(this.Points_, function (Point) {
        // draw little red circle
        canvas.beginPath();
        canvas.arc(Point.x, Point.y, 3, 0, 2 * Math.PI, false);
        canvas.closePath();
        canvas.fill();
    });
};

CalibrationPoints.prototype.sendData = function () {
    console.log('sendData event');

    // TODO: This is buggy because of my less knowledge of ofa and javascript

    //TODO: JSON DATA has to be modified
    var _PointData = _.map(this.Points_, function (Point) {
        return _.pick(Point, 'x', 'y');
    });
    console.log(_PointData);

    var _senddata =
        [
            {mp: this.config.mountPoint, key: this.config.key, value: _PointData}
        ];

    ioSocket.emit('config.set', _senddata);

    _.each(this.Points_, function (Point) {
        Point.button.parentNode.removeChild(Point.button);
    });
    this.Points_ = [];
};

CalibrationPoints.prototype.onPointClick = function (point, event) {
    // Remove Point from Array
    this.Points_.splice(this.Points_.indexOf(point), 1);
    // Remove button
    point.button.parentNode.removeChild(point.button);
};

function findOffset(obj) {
    var ObjLeft = 0;
    var ObjTop = 0;
    //if (obj.offsetParent)
    {
        do {
            ObjLeft += (obj.offsetLeft - obj.scrollLeft + obj.clientLeft);
            ObjTop += (obj.offsetTop - obj.scrollTop + obj.clientTop);
        } while (obj = obj.offsetParent)
    }
    return {x: ObjLeft, y: ObjTop};
}

module.exports = CalibrationPoints;
