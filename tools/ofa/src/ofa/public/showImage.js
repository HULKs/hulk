/**
 * Created by finn on 07.02.15.
 */

var ImageStream = function() {
	this.init.apply(this, arguments);
};
util.inherits(ImageStream, DebugDisplay);

ImageStream.playing = true;

ImageStream.defaultConfig = {
	title:'',
	frameSelect: 1,
	keepRatio: true
};

ImageStream.expectedKeys = [['image']];
ImageStream.filter = ['image'];

ImageStream.prototype.init = function(config){
	DebugDisplay.prototype.init.call(this, config);

	this.config = _.defaults(config||{}, ImageStream.defaultConfig);

	this.frameCount = 0;
	this.frameSelect = parseInt(this.config.frameSelect) || 1; //default to 1, if frameSelect cannot be parsed as an integer or is 0

	this.wrapper.classList.add('image');

	if(!this.controllbtnstop) {
		this.controllbtnstop = document.createElement('div');
		this.controllbtnstop.addEventListener('click', _.bind(this.togglePlaying, this),false);
		this.head.appendChild(this.controllbtnstop);
	}

	this.controllbtnstop.isToggled = false;
	this.controllbtnstop.className = "btn space";
	this.controllbtnstop.innerHTML = "Stop";
	this.controllbtnstop.title = "Stop image sequence";

	this.canvas = document.createElement('canvas');
	this.canvas.width = 640;
	this.canvas.imageratioW2H = 640/480;
	this.canvas.height = this.canvas.width / this.canvas.imageratioW2H;
	this.wrapper.appendChild(this.canvas);

	this.ctx = this.canvas.getContext('2d');

	this.nextImg = null;
	this.loadingImg = null;
	this.onImageLoad = _.bind(this.onImageLoad, this);
	this.paint = _.bind(this.paint, this);

	this.onUpdate = _.bind(this.onUpdate, this);
	this.subscription = debugMan.subscribeImage(config.keys[0], this.onUpdate);
	requestAnimationFrame(this.paint);
};

ImageStream.prototype.onUpdate = function(image){
	if (ImageStream.playing == false) return;
	if (this.frameCount++ % this.frameSelect !== 0) return;

	this.loadingImg = new Image();
	this.loadingImg.src = '/image/' + image + '?' + new Date().getTime();
	this.loadingImg.addEventListener('load', this.onImageLoad);
};

ImageStream.prototype.onImageLoad = function() {
	this.nextImg = this.loadingImg;
};

ImageStream.prototype.paint = function() {
	requestAnimationFrame(this.paint);
	if (this.nextImg === null) return;
	this.ctx.drawImage(this.nextImg, 0, 0, this.canvas.width, this.canvas.height);
	this.nextImg = null;
};

ImageStream.prototype.togglePlaying = function() {
	if (this.controllbtnstop.isToggled === true) {
		ImageStream.playing = true;
		this.controllbtnstop.innerHTML = "Stop";
		this.controllbtnstop.title = "Stop image sequence";
	} else {
		ImageStream.playing = false;
		this.controllbtnstop.innerHTML = "Play";
		this.controllbtnstop.title = "Play image sequence";
	}
	this.controllbtnstop.isToggled = !this.controllbtnstop.isToggled;
};

ImageStream.prototype.resize = function(size){
	if (this.config.keepRatio) {
		this.canvas.width = size[0];
		this.canvas.height = size[0] / this.canvas.imageratioW2H;
	} else {
		this.canvas.width = size[0];
		this.canvas.height = size[1];
	}
};
