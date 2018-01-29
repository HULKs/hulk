function DebugDisplay() {
	this.init.apply(this, arguments);
}
util.inherits(DebugDisplay, Panel);

DebugDisplay.prototype.init = function(config) {
	this._lastInitConfig = config;
	if ( ! this.panel ) {
		Panel.prototype.init.call(this, config);

		var editBtn = this._editBtn = document.createElement('div');
		editBtn.className = 'btn';
		editBtn.textContent = 'Edit';
		editBtn.addEventListener('click', _.bind(this.onEdit, this), false);
		this.head.appendChild(editBtn);
	}
};

DebugDisplay.prototype.onEdit = function() {
	configModal.show(this.constructor, this);
};

DebugDisplay.prototype.reInit = function(config) {
	if ( this.subscription ) {
		debugMan.unsubscribe(this.subscription);
		this.subscription = null;
	}

	this._titleSpan.textContent = config.title + '\u00A0';
	this.wrapper.innerHTML = '';

	this.init(config);
};

DebugDisplay.prototype.subscribe = function(keys, mappingFct) {
	this.subscription = debugMan.subscribe(keys, mappingFct, this.onUpdate);
};

DebugDisplay.prototype.onUpdate = function() {};

DebugDisplay.prototype.onClose = function() {
	if ( this.subscription ) {
		debugMan.unsubscribe(this.subscription);
		this.subscription = null;
	}
	Panel.prototype.onClose.call(this);
};
