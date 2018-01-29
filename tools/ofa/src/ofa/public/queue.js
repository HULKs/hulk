var Queue = function() {
	this.init.apply(this, arguments);
};
util.inherits(Queue, DebugDisplay);

Queue.defaultConfig = {
	title:''
};
Queue.expectedKeys = [['[string]']];

Queue.prototype.init = function(config) {
	DebugDisplay.prototype.init.call(this, config);

	console.log('graph init', JSON.stringify(config));
	this.onUpdate = _.bind(this.onUpdate, this);

	this.config = _.defaults(config||{}, Queue.defaultConfig);

	this.wrapper.classList.add('queue');

	this.textBox = document.createElement('textarea');
	this.wrapper.appendChild(this.textBox);

	this.subscribe(config.keys, config.mappingFct);
};

Queue.prototype.onUpdate = function(val) {
	var __self = this;
	_.each(val, function(message) {
		var line = document.createTextNode(message+'\n');
		__self.textBox.appendChild(line);
	});
	this.textBox.scrollTop = this.textBox.scrollHeight;
};

