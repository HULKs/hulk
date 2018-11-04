var _ = require('underscore');
var util = require('./ui').util;
var DebugDisplay = require('./debugDisplay');

var Graph = function () {
    this.init.apply(this, arguments);
};
util.inherits(Graph, DebugDisplay);

Graph.defaultConfig = {
    title: '',
    width: 500,
    height: 101,
    lineColor: 'white',
    bufferSize: 100,
    minValue: -2,
    maxValue: 2,
    autoScale: true
};
Graph.expectedKeys = [['float'], ['[float]']];

Graph.prototype.init = function (config) {
    DebugDisplay.prototype.init.call(this, config);

    console.log('graph init', JSON.stringify(config));
    this.onUpdate = _.bind(this.onUpdate, this);

    this.config = _.defaults(config || {}, Graph.defaultConfig);
    this.config.bufferSize = parseInt(this.config.bufferSize);
    this.config.minValue = parseFloat(this.config.minValue);
    this.config.maxValue = parseFloat(this.config.maxValue);
    this.showMinMaxLegend = (this.config.autoScale) ? true : false;

    this.wrapper.classList.add('graph');

    this.canvas = document.createElement('canvas');
    this.canvas.width = this.config.width;
    this.canvas.height = this.config.height;
    this.wrapper.appendChild(this.canvas);

    this.ctx = this.canvas.getContext('2d');
    this.ctx.strokeStyle = this.config.lineColor;

    this.buf = new Array(this.config.bufferSize);


    if (_.isArray(this.config.returnType)) {
        this.paintData = _.bind(this.paintArray, this)
        this.updateData = _.bind(this.updateArrayData, this)

        this.buf = new Array(config.returnType.length);
        for (var i = 0; i < config.returnType.length; i++) {
            this.buf[i] = new Array(this.config.bufferSize);
        }
    } else {
        this.paintData = _.bind(this.paintScalar, this)
        this.updateData = _.bind(this.updateScalarData, this)

        this.buf = new Array(this.config.bufferSize);
    }

    this.subscribe(config.keys, config.mappingFct);
    requestAnimationFrame(_.bind(this.paint, this));
};

Graph.prototype.onUpdate = function (val) {
    this.updateData(val)
};

Graph.prototype.updateArrayData = function (val) {
    for (var i = 0; i < this.buf.length; i++) {
        this.buf[i].shift();
        this.buf[i].push(val[i]);
        if (this.config.autoScale) {
            this.config.minValue = Math.floor(Math.min(val[i], this.config.minValue))
            this.config.maxValue = Math.ceil(Math.max(val[i], this.config.maxValue))
        }
    }
}

Graph.prototype.updateScalarData = function (val) {
    this.buf.shift();
    this.buf.push(val);
    if (this.config.autoScale) {
        this.config.minValue = Math.floor(Math.min(val, this.config.minValue))
        this.config.maxValue = Math.ceil(Math.max(val, this.config.maxValue))
    }
};

Graph.prototype.paint = function () {
    this.ctx.fillRect(0, 0, this.config.width, this.config.height);
    if (this.showMinMaxLegend === true) {
        this.ctx.strokeStyle = "#FFFFFF"
        var min = this.config.minValue.toFixed(0)
        var max = this.config.maxValue.toFixed(0)
        this.minMaxLegendText = min + " : " + max
        this.ctx.strokeText(this.minMaxLegendText, 10, 10)
    }
    this.ctx.strokeStyle = this.config.lineColor;

    this.paintData()

    requestAnimationFrame(_.bind(this.paint, this));
};

Graph.prototype.normalize = function (val) {
    return (1 - (val - this.config.minValue) / (this.config.maxValue - this.config.minValue)) * this.config.height;
};

Graph.prototype.paintArray = function () {
    var stepInc = this.config.width / (this.config.bufferSize - 1);
    for (var i = 0; i < this.buf.length; i++) {
        if (this.buf.length <= 3) {
            switch (i) {
                case 0:
                    this.ctx.strokeStyle = "#FF0000"
                    break;
                case 1:
                    this.ctx.strokeStyle = "#44FF44"
                    break;
                case 2:
                    this.ctx.strokeStyle = "#aaaaFF"
                    break;
            }
        } else {
            var scaled = Math.round((255 / this.buf.length) * i);
            var color = colormap_rainbow[scaled];
            this.ctx.strokeStyle = 'rgba(' + color.join(',') + ')';
            ;
        }

        this.ctx.beginPath();
        this.ctx.moveTo(0, this.normalize(this.buf[i][0]));
        for (var x = 1; x < this.config.bufferSize; x++) {
            this.ctx.lineTo(x * stepInc, this.normalize(this.buf[i][x]));
        }
        this.ctx.stroke();
        var text = this.buf[i][this.buf[i].length - 1];
        if (text !== undefined) {
            text = text.toFixed(3);
        }
        var legendTextWidth = this.ctx.measureText(this.minMaxLegendText).width
        this.ctx.strokeText(text, legendTextWidth + (i + 1) * ((this.config.width - 10 - legendTextWidth) / this.buf.length) - this.ctx.measureText(text).width, 10);
    }
}

Graph.prototype.paintScalar = function () {
    this.ctx.beginPath();
    this.ctx.moveTo(0, this.normalize(this.buf[0]));
    var stepInc = this.config.width / (this.config.bufferSize - 1);
    for (var x = 1; x < this.config.bufferSize; x++) {
        this.ctx.lineTo(x * stepInc, this.normalize(this.buf[x]));
    }
    this.ctx.stroke();
    var text = this.buf[this.buf.length - 1];
    if (text !== undefined) {
        text = text.toFixed(3);
    }
    this.ctx.strokeText(text, this.config.width - this.ctx.measureText(text).width - 10, 10);
}

Graph.prototype.normalize = function (val) {
    return (1 - (val - this.config.minValue) / (this.config.maxValue - this.config.minValue)) * this.canvas.height;
};

Graph.prototype.resize = function (size) {
    this.canvas.width = size[0]
    this.canvas.height = size[1]
    this.config.width = size[0]
    this.config.height = size[1]
}

module.exports = Graph
