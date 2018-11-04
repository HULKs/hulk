var _ = require('underscore');
var util = require('./ui').util;
var DebugDisplay = require('./debugDisplay');

var XYGraph = function () {
    this.init.apply(this, arguments);
};
util.inherits(XYGraph, DebugDisplay);

XYGraph.defaultConfig = {
    title: 'XYGraph',
    size: 501,
    lineColor: 'white',
    bufferSize: 50,
    maxValue: 100
};
XYGraph.expectedKeys = [['float', 'float']];

XYGraph.prototype.init = function (config) {
    DebugDisplay.prototype.init.call(this, config);

    this.onUpdate = _.bind(this.onUpdate, this);

    this.config = _.defaults(config || {}, XYGraph.defaultConfig);
    this.config.bufferSize = parseInt(this.config.bufferSize);
    this.config.minValue = parseFloat(this.config.minValue);
    this.config.maxValue = parseFloat(this.config.maxValue);

    this.wrapper.classList.add('graph');

    this.canvas = document.createElement('canvas');
    this.canvas.width = this.config.size;
    this.canvas.height = this.config.size;
    this.wrapper.appendChild(this.canvas);

    this.ctx = this.canvas.getContext('2d');
    this.ctx.strokeStyle = this.config.lineColor;

    this.buf = new Array(this.config.bufferSize);

    for (var i = 0; i < this.config.bufferSize; i++) {
        this.buf[i] = [0, 0];
    }

    this.subscribe(config.keys, config.mappingFct);
    requestAnimationFrame(_.bind(this.paint, this));
};

XYGraph.prototype.onUpdate = function (x, y) {
    this.buf.shift();
    this.buf.push([x, y]);
};

XYGraph.prototype.paint = function () {
    this.ctx.fillRect(0, 0, this.config.size, this.config.size);

    this.ctx.beginPath();

    this.ctx.moveTo(this.normalize(this.buf[0][0]), this.normalize(this.buf[0][1]));


    for (var i = 1; i < this.config.bufferSize; i++) {
        this.ctx.lineTo(this.normalize(this.buf[i][0]), this.normalize(this.buf[i][1]));
    }

    var lastElem = this.buf[this.config.bufferSize - 1];

    this.ctx.moveTo(0, this.normalize(lastElem[1]));
    this.ctx.lineTo(this.config.size, this.normalize(lastElem[1]));
    this.ctx.moveTo(this.normalize(lastElem[0]), 0);
    this.ctx.lineTo(this.normalize(lastElem[0]), this.config.size);

    this.ctx.stroke();
    this.ctx.strokeText(lastElem[1], 10, 20);
    this.ctx.strokeText(lastElem[0], 10, 35);

    requestAnimationFrame(_.bind(this.paint, this));
};

XYGraph.prototype.normalize = function (val) {
    return (1 - val / this.config.maxValue) * this.config.size >> 1;
};

XYGraph.prototype.resize = function (size) {
    this.config.size = Math.min(size[0], size[1])
    this.canvas.width = this.config.size
    this.canvas.height = this.config.size
    this.ctx.strokeStyle = this.config.lineColor;
}

module.exports = XYGraph;
