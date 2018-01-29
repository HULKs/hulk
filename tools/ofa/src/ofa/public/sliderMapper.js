var sliderMapper = {};
sliderMapper.modal = new ui.Modal('Map Slider');

sliderMapper.show = function(defaults) {
	defaults = defaults || {};
	var body = this.modal.body;
	body.innerHTML = '';

	var sliderDD = this.sliderSelect = document.createElement('select');
	_.each(_.range(8).concat(_.range(16,24)), function(slider) {
		var elem = document.createElement('option');
		elem.value = slider;
		elem.appendChild(document.createTextNode(slider));
		sliderDD.appendChild(elem);
	});
	body.appendChild(sliderDD);

	var configOptions = {
		'mp': 'MountPoint',
		'key': 'Key',
		'min': 'Min Value',
		'max': 'Max Value'
	};
	_.defaults(defaults, {
		'mp': '',
		'key': '',
		'min': '',
		'max': ''
	})
	this.configFields = _.map(configOptions, _.bind(function(labelText, name) {
		var id = _.uniqueId('configField');
		var label = document.createElement('label');
		label.for = id;
		label.appendChild(document.createTextNode(labelText));
		body.appendChild(label);
		var input = document.createElement('input');
		input.id = id;
		input.name = name;
		input.value = defaults[name];
		body.appendChild(input);
		return input;
	}, this));

	// Create putains
	ui.createButton(body, 'OK', this.onOK, this);
	ui.createButton(body, 'Cancel', this.onCancel, this);

	this.keyPressBinding = _.bind(this.onKeyUp, this)
	document.documentElement.addEventListener("keyup", this.keyPressBinding, false);

	// Show Modal
	this.modal.show();
};

sliderMapper.hide = function() {
	document.documentElement.removeEventListener("keyup", this.keyPressBinding, false);
	this.modal.hide();
};

sliderMapper.onKeyUp = function (e) {
	e = e || window.event;
	if (e.keyCode == 13) { // detect enter
		this.onOK();
		return false;
	}else if (e. keyCode == 27) { // detect esc
		this.onCancel()
		return false;
	}
	return true;
}

sliderMapper.onOK = function() {
	var channel = parseInt(this.sliderSelect.value);
	var config = {
		ch: channel
	};
	_.each(this.configFields, function(field) {
		config[field.name] = field.value;
	});
	config.max = parseFloat(config.max);
	config.min = parseFloat(config.min);

	ioSocket.emit('config.map', config);
	this.hide();
};

sliderMapper.onCancel = function() {
	this.hide();
};
