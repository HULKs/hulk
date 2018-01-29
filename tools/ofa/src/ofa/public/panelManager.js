function PanelManager() {
	EventEmitter2.call(this);
	this.openPanels = [];
	this.positionOffsetIndex = 0;
}
util.inherits(PanelManager, EventEmitter2);

PanelManager.prototype.register = function(panel) {
	if ( this.openPanels.indexOf(panel) > -1 ) {
		console.error('panel already registered');
		return;
	}
	this.openPanels.push(panel);
	this.notifyUpdate();
};

PanelManager.prototype.unregister = function(panel) {
	var pos = this.openPanels.indexOf(panel);
	if ( this.openPanels.indexOf(panel) == -1 ) {
		console.error('panel not registered');
		return;
	}
	this.openPanels.splice(pos, 1);
	this.notifyUpdate();
};

PanelManager.prototype.notifyUpdate = function() {
	this.emit('change');
};

PanelManager.prototype.clear = function() {
	var panelList = _.clone(this.openPanels);
	_.each(panelList, function(p) { p.onClose(); });
};

PanelManager.prototype.openPanelsToJSONString = function() {
	var desc = _.map(this.openPanels, function(p) {
		return p.getDescription();
	});
	return JSON.stringify(desc, null, '\t');
};

PanelManager.prototype.openFromJSONString = function(str) {
	this.clear();
	var panelData = JSON.parse(str);
	_.each(panelData, function(data) {
		var ctr = panels[data.panel];
		if ( ctr.prototype instanceof DebugDisplay ) {
			// create mappingFct from keys and mapping string.
			// TODO: duplicate code with configDialog.js onOK fct. Extract!
			(function() {
				var argCount = data.config.keys.length;
				var argNames = [];
				for ( var i = 0; i < argCount; i++ ) {
					argNames.push('key'+i);
				}
				try {
					data.config.mappingFct = Function.apply({}, argNames.concat(data.config.mapping));
				} catch(e) {
					alert('Mapping function error: '+e.message);
					return null;
				}
			}());
		}
		var instance = new ctr(data.config);
		instance.setPosition(data.position);
		if ( data.resize ) {
			instance.resize(data.resize);
		}
	});
};

PanelManager.prototype.getNextPositionOffset = function() {
	var offset = this.positionOffsetIndex * ui.gridsize * 2;
	this.positionOffsetIndex = ++this.positionOffsetIndex % 8;
	return [ui.viewportOffset.x + offset, ui.viewportOffset.y + offset];
};

PanelManager.getPanelName = function(instance) {
    var name = null;
    _.find(panels, function(panel, key) {
        if ( panel == instance.constructor ) {
            name = key;
            return true;
        }
    });
    return name;
};
