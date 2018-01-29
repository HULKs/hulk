function Panel() {
	this.init.apply(this, arguments);
}

Panel.prototype.init = function(config) {
	this._lastInitConfig = config;

	this.panel = document.createElement('div');
	this.panel.className = 'panel';

	this.head = document.createElement('section');
	this.head.className = 'head';
	this.head.addEventListener('mousedown', _.bind(this.startMove, this), false);
	this.panel.appendChild(this.head);

	var closeBtn = this._closeBtn = document.createElement('div');
	closeBtn.className = 'close btn';
	closeBtn.addEventListener('click', _.bind(this.onClose, this), false);
	this.head.appendChild(closeBtn);

	var title = this._titleSpan = document.createElement('span');
	title.textContent = config.title + '\u00A0';
	this.head.appendChild(title);

	this.wrapper = document.createElement('div');
	this.wrapper.className = 'body';
	this.panel.appendChild(this.wrapper);

	if ( this.resize ) {
		this.resizer = document.createElement('div');
		this.resizer.className = 'resizer';
		this.resizer.addEventListener('mousedown', _.bind(this.startResize, this), false);
		this.panel.appendChild(this.resizer);

		// Wrap resize function to fetch and store latest size update
		var __this = this;
		var resizeImpl = this.resize;
		this.resize = function(size) {
			__this._lastResize = size;
			resizeImpl.call(this, size);
		};
	}

	ui.body.appendChild(this.panel);
	this.setPosition(panelManager.getNextPositionOffset());
	panelManager.register(this);
};

Panel.prototype.setPosition = function(coords) {
	this.panel.style.left = Math.max(ui.viewportOffset.x, coords[0]) + 'px';
	this.panel.style.top  = Math.max(ui.viewportOffset.y, coords[1]) + 'px';
}

Panel.prototype.onClose = function() {
	this.panel.parentNode.removeChild(this.panel);
	panelManager.unregister(this);
};

Panel.prototype.startResize = function(start) {
	var initialWidth = this.panel.clientWidth;
	var initialHeight = this.panel.clientHeight - this.head.clientHeight;

	var resize = _.bind(function(move) {
		var newWidth = ui.snapToGrid(initialWidth + move.clientX - start.clientX);
		var newHeight = ui.snapToGrid(initialHeight + move.clientY - start.clientY);
		this.resize([newWidth, newHeight]);
		move.preventDefault();
	}, this);

	var up = _.bind(function() {
		document.documentElement.removeEventListener('mousemove', resize);
		document.documentElement.removeEventListener('mouseup', up);

		var newWidth = this.panel.clientWidth;
		var newHeight = this.panel.clientHeight - this.head.clientHeight;
		if (newWidth != initialWidth || newHeight != initialHeight) {
			panelManager.notifyUpdate();
		}
	}, this);

	document.documentElement.addEventListener('mousemove', resize, false);
	document.documentElement.addEventListener('mouseup', up, false);
}

Panel.prototype.startMove = function(start) {
	var initialLeft = this.panel.offsetLeft;
	var initialTop = this.panel.offsetTop;

	var move = _.bind(function(move) {
		var newLeft = ui.snapToGrid(initialLeft + move.clientX - start.clientX);
		var newTop = ui.snapToGrid(initialTop + move.clientY - start.clientY);
		this.setPosition([newLeft, newTop]);
		move.preventDefault();
	}, this);
	var up = _.bind(function() {
		document.documentElement.removeEventListener('mousemove', move);
		document.documentElement.removeEventListener('mouseup', up);

		var newLeft = this.panel.offsetLeft;
		var newTop = this.panel.offsetTop;
		if (newLeft != initialLeft || newTop != initialTop) {
			panelManager.notifyUpdate();
		}
	}, this);

	document.documentElement.addEventListener('mousemove', move, false);
	document.documentElement.addEventListener('mouseup', up, false);
}

Panel.prototype.getDescription = function() {
	return {
		panel: PanelManager.getPanelName(this),
		config: this._lastInitConfig,
		position: [this.panel.offsetLeft, this.panel.offsetTop],
		resize: this._lastResize
	}
}
