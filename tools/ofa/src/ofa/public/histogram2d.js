var Histogram2d = function() {
	this.init.apply(this, arguments);
};
util.inherits(Histogram2d, DebugDisplay);

Histogram2d.defaultConfig = {
	title:'',
	width:256,
	height:256,
	bufferSizeX: 256,
	bufferSizeY: 256
};
Histogram2d.expectedKeys = ['[float]'];

Histogram2d.prototype.init = function(config) {
	DebugDisplay.prototype.init.call(this, config);

	console.log('Histogram2d init', JSON.stringify(config));
	this.onUpdate = _.bind(this.onUpdate, this);

	this.config = _.defaults(config||{}, Histogram2d.defaultConfig);
	this.config.bufferSize = parseInt(this.config.bufferSize);
	this.config.minValue = parseFloat(this.config.minValue);
	this.config.maxValue = parseFloat(this.config.maxValue);

	this.wrapper.classList.add('Histogram2d');

	this.canvas = document.createElement('canvas');
	this.canvas.width = this.config.width;
	this.canvas.height = this.config.height;
	this.wrapper.appendChild(this.canvas);

	this.ctx = this.canvas.getContext('2d');
	this.ctx.strokeStyle = this.config.lineColor;

	this.buf = new Array(this.config.bufferSizeX*this.config.bufferSizeY);

	this.subscribe(config.keys, config.mappingFct);
	requestAnimationFrame(_.bind(this.paint, this));
};

Histogram2d.prototype.onUpdate = function(val) {
  this.buf = val;
};

Histogram2d.prototype.paint = function() {
  var imageData = this.ctx.createImageData(256,256);
  var max_value = Math.log(640*480);
	for ( var y = 0; y < this.config.bufferSizeX; y++ ) {
    for ( var x = 0; x < this.config.bufferSizeY; x++ ) {
      var index = y * this.config.bufferSizeY + x;
      var histo_data = this.buf[index] == 0 ? 0 : Math.log(this.buf[index]);
      var histo_data = isNaN(histo_data) ? 0 : histo_data;
      var scaled = Math.round(histo_data/max_value * 255);
      var color = colormap_portland[scaled];

      var imageIndex = (255-y) * this.config.bufferSizeY + x;
      imageData.data[imageIndex*4+0] = color[0];
      imageData.data[imageIndex*4+1] = color[1];
      imageData.data[imageIndex*4+2] = color[2];
      imageData.data[imageIndex*4+3] = 255;
    }
  }
  this.ctx.putImageData(imageData,0,0);
	requestAnimationFrame(_.bind(this.paint, this));
};

Histogram2d.prototype.normalize = function(val) {
	return (1 - (val - this.config.minValue) / (this.config.maxValue - this.config.minValue))*this.config.height;
};
