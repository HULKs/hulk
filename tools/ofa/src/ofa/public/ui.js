Node.prototype.addText = function(text) {
	this.appendChild(document.createTextNode(text));
	return this;
};

var ui = {
	gridsize: 10,
	viewportOffset: { x:0, y:30 },
	body: document.getElementsByTagName('body')[0],
	createButton: function(container, caption, handler, context) {
		var button = document.createElement('button');
		button.appendChild(document.createTextNode(caption));
		button.addEventListener('click', _.bind(handler, context || this), false);
		container.appendChild(button);
		return button;
	},

	createWrapper: function(className) {
		var wrapper = document.createElement('div');
		wrapper.className = className;
		ui.body.appendChild(wrapper);
		return wrapper;
	},

	snapToGrid: function(x) { return Math.round(x / ui.gridsize) * ui.gridsize; }
};

var util = {
	inherits: function(ctor, superCtor) {
		ctor.super_ = superCtor;
		ctor.prototype = Object.create(superCtor.prototype, {
			constructor: {
				value: ctor,
				enumerable: false,
				writable: true,
				configurable: true
			}
		});
	}
};

function Navigation() {
	this.topbar = document.createElement('nav');
	this.topbar.className = 'topbar';
	ui.body.appendChild(this.topbar);

	this.navList = document.createElement('ul');
	this.topbar.appendChild(this.navList);

	this.addPanelBtn = document.createElement('div');
	this.addPanelBtn.className = 'addPanelBtn';
	this.addPanelBtn.addEventListener('click', _.bind(this.onAddPanelBtnClick, this));
	ui.body.appendChild(this.addPanelBtn);
	ui.body.addEventListener('click', _.bind(this.onBodyClick, this));

	this.panelPopout = document.createElement('nav');
	this.panelPopout.className = 'panelList';
	ui.body.appendChild(this.panelPopout);

	this.panelList = document.createElement('ul');
	this.panelPopout.appendChild(this.panelList);

	for(var name in panels){
		var panelType = panels[name];
		var item = document.createElement('li');
		item.appendChild(document.createTextNode(name));
		item.addEventListener('click', _.bind(this.onPanelTypeClick, this, panelType));
		this.panelList.appendChild(item);
	}

	this.addNavItem('Disconnect', _.bind(connectionManager.disconnect, connectionManager))
	this.addNavItem('Map Slider', _.bind(sliderMapper.show, sliderMapper))
};

Navigation.prototype.addNavItem = function(caption, handler) {
	var item = document.createElement('li');
	item.appendChild(document.createTextNode(caption));
	item.addEventListener('click', handler);
	this.navList.appendChild(item);
	return item;
}

Navigation.prototype.onAddPanelBtnClick = function(e) {
	this.panelPopout.classList.toggle('open');
	e.stopPropagation();
};

Navigation.prototype.onBodyClick = function() {
	this.panelPopout.classList.remove('open');
}

Navigation.prototype.onPanelTypeClick = function(panelType) {
	if ( panelType.prototype instanceof DebugDisplay ) {
		configModal.show.call(configModal, panelType, null);
	} else {
		new panelType();
	}
};

ui.ModalOverlay = (function() {
	var elem = document.createElement('div');
	elem.className = 'modalOverlay';
	ui.body.appendChild(elem);

	var Overlay = {};
	Overlay.hide = function() {
		elem.style.display = 'none';
	};
	Overlay.show = function() {
		elem.style.display = '';
	};
	Overlay.hide();
	return Overlay;
}());
ui.Modal = function Modal(caption) {
	this.container = document.createElement('div');
	this.container.className = 'modal';
	this.container.style.top = '20px';
	this.hide();
	this.head = document.createElement('section');
	this.head.className = 'head';
	this.head.textContent = caption;
	this.body = document.createElement('section');
	this.body.className = 'body';

	this.container.appendChild(this.head);
	this.container.appendChild(this.body);
	ui.body.appendChild(this.container);
};
ui.Modal.prototype.hide = function() {
	ui.ModalOverlay.hide();
	this.container.style.display = 'none';
};
ui.Modal.prototype.show = function() {
	ui.ModalOverlay.show();
	this.container.style.display = '';
};
