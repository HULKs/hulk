var ioSocket = io();
ioSocket.on('configConnect', function() {
	connectionManager.configConnected = true;
	connectionManager.updateConnect();
});
ioSocket.on('debugConnect', function() {
	connectionManager.debugConnected = true;
	connectionManager.updateConnect();
});
ioSocket.on('debugDisconnect', function() {
	connectionManager.debugConnected = false;
	connectionManager.updateConnect();
	connectionManager.disconnect();
});
ioSocket.on('configDisconnect', function() {
	connectionManager.configConnected = false;
	connectionManager.updateConnect();
});
ioSocket.on('debugError', function(msg) {
	alert('Debug protocol error: "'+msg+'"');
});
ioSocket.on('configError', function(msg) {
	alert('Config protocol error: "'+msg+'"');
});
ioSocket.on('colormap_portland', function(map) {
    colormap_portland = map;
});
ioSocket.on('colormap_rainbow', function(map) {
    colormap_rainbow = map;
});

var colormap_rainbow = null;
var colormap_portland = null;
var connectionManager = {};

connectionManager.init = function(defaultNaos, hostname) {
	this.modal = new ui.Modal('Connect to NAO');
	this.modal.container.classList.add('medium');
	this.configConnected = false;
	this.debugConnected = false;
	this.modalOpen = false;
	this.alive = {};
	this.defaultNaos = defaultNaos;

	if ( hostname ) {
		ioSocket.emit('connectNao', hostname);
	} else {
		this.show();
	}
};

connectionManager.show = function() {
	var body = this.modal.body;
	body.innerHTML = '';

	var elem = document.createElement('div');
	elem.className = 'robotBtn';
	elem.innerHTML = 'Enter NAO hostname ...';
	elem.addEventListener('click', _.bind(this.connect, this, ''), false);
	body.appendChild(elem);

	var socketBtn = document.createElement('div');
	socketBtn.className = 'robotBtn';
	socketBtn.innerHTML = 'Enter SimRobot Socket Path ...';
	socketBtn.addEventListener('click', _.bind(this.connectLocal, this, ''), false);
	body.appendChild(socketBtn);

	this.naoContainer = document.createElement('div');
	this.naoContainer.className = 'robotList';
	body.appendChild(this.naoContainer);

	// Show Modal
	this.modal.show();
	this.modalOpen = true;
	this.refreshNaoList();
};

connectionManager.hide = function() {
	this.modalOpen = false;
	this.modal.hide();
};

connectionManager.updateConnect = function() {
	var connected = this.configConnected || this.debugConnected;
	if ( ! connected ) {
		this.show();
	}
};

connectionManager.updateAlive = function(alive) {
	this.alive = alive;
	if ( ! this.modalOpen ) return;
	this.refreshNaoList();
}

connectionManager.refreshNaoList = function() {
	var robots = {};
	for (var ip in this.defaultNaos) {
		robots[ip] = {
			ip : ip,
			name : this.defaultNaos[ip],
			active : false
		};
	}
	for (var ip in this.alive) {
		var info = this.alive[ip];
		robots[ip] = {
			ip : ip,
			name : info.head + ' / ' + info.body,
			active : true
		}
	}

	this.naoContainer.innerHTML = '';
	for (var ip in robots) {
		var info = robots[ip];
		var elem = document.createElement('div');
		elem.className = 'robotBtn' + (info.active ? ' active' : '');
		elem.innerHTML = info.name + '<div class="ip">' + ip + '</div>';
		elem.addEventListener('click', _.bind(this.connect, this, ip), false);
		this.naoContainer.appendChild(elem);
	}
};

connectionManager.connect = function(hostname) {
	if (!hostname) {
		hostname = prompt('Enter Nao Hostname', 'localhost');
	}
	ioSocket.emit('connectNao', hostname);
	this.hide();
};

connectionManager.connectLocal = function(path) {
	if (!path) {
		path = prompt('Enter SimRobot Socket path', '/tmp/simrobot/robot2');
	}
	ioSocket.emit('connectLocal', path);
	this.hide();
};

connectionManager.disconnect = function() {
	ioSocket.emit('disconnectNao');
};

ioSocket.on('alive', function(alive) {
	connectionManager.updateAlive(alive);
});

var configMan = {};
configMan.set = function(mp, key, value) {
	ioSocket.emit('config.set', [{ mp:mp, key:key, value:value }]);
};
configMan.getMounts = function(cb) {
	ioSocket.once('config.sendMounts', function(data) { cb(data.keys); });
	ioSocket.emit('config.getMounts');
};
configMan.getKeys = function(mountPoint, cb) {
	var checker = function(data) {
		if ( data.mountPoint != mountPoint ) return;
		cb(data.keys);
		ioSocket.removeListener('config.sendKeys', checker);
	};
	ioSocket.on('config.sendKeys', checker);
	ioSocket.emit('config.getKeys', mountPoint);
};
configMan.save = function() {
	ioSocket.emit('config.save');
};

var debugMan = { keyList:{}, subscriptions:{}, keySubCount:{}, data:{}, connectionId:''};
debugMan.init = function() {
	ioSocket.on('debugConnect', function(id) {
		ioSocket.emit('debug.listCommands');
		debugMan.connectionId = id;
		debugMan.resubscribe();
	});
	ioSocket.on('list', _.bind(function(data) {
		this.keyList = _.foldl(data.keys, function(map, key) {
			map[key.key] = key;
			return map;
		}, {});
	}, this));

	ioSocket.on('update', _.bind(this.onUpdate, this));
	ioSocket.on('image', _.bind(this.onImage, this));
};

debugMan.onUpdate = function(updates) {
	for ( var i = 0; i < updates.length; i++ ) {
		var update = updates[i];
		this.data[update.key] = update;
	}
	var data = this.data;
	_.each(this.subscriptions, function(sub) {
		if ( sub.isImage ) return;
		var values = _.map(sub.keys, function(key) {
			if (!data || !data.hasOwnProperty(key[0])) {
				throw new Error('Subscribed to undefined key: ' + key);
			};
			var value = data[key[0]].value;
			return ( key.length == 2 && typeof value == 'object' ) ? value[key[1]] : value;
		});
		sub.handler.apply(sub, sub.mapping.apply(sub, values));
	});
};

debugMan.onImage = function (imageKey) {
	_.each(this.subscriptions, function(sub) {
		if(sub.isImage && sub.key == imageKey) {
			sub.handler(debugMan.connectionId+'-'+sub.key);
		}
	});
};

debugMan._unifyKey = function(key) {
	if ( typeof key === 'object' ) return key;
	if ( typeof key !== 'string' ) throw new Error('invalid key format');
	var match = key.match(/([^\]]*)(\[([0-9]+)\])?$/);
	if ( match === null ) throw new Error('invalid key format');
	if ( typeof match[3] === 'undefined' )
		return [match[1]];
	// else
		return [match[1], parseInt(match[3])];
};
debugMan._subscribeKey = function(key) {
	console.log('subscribeKey', key);
	var keyName = key[0];
	if ( ! this.keySubCount.hasOwnProperty(keyName) ) {
		if ( connectionManager.debugConnected )
			ioSocket.emit('debug.subscribe', keyName);
		this.keySubCount[keyName] = 0;
	}
	this.keySubCount[keyName]++;
};
debugMan._unsubscribeKey = function(key) {
	console.log('unsubscribeKey', key);
	var keyName = key[0];
	if ( ! this.keySubCount.hasOwnProperty(keyName) ) return;
	this.keySubCount[keyName]--;
	if ( this.keySubCount[keyName] == 0 ) {
		ioSocket.emit('debug.unsubscribe', keyName);
		delete this.keySubCount[keyName];
	}
};

debugMan.resubscribe = function() {
	var keys = _.keys(this.keySubCount);
	ioSocket.emit('debug.subscribeBulk', keys);
};

debugMan.subscribe = function(keys, mapping, handler) {
	if ( typeof keys !== 'object' ) { throw new Error('subscribe expects an array of keys as first parameter'); }
	keys = _.map(keys, _.bind(this._unifyKey, this));
	_.each(keys, _.bind(this._subscribeKey, this));
	var subId = _.uniqueId('sub');
	this.subscriptions[subId] = {
		keys: keys,
		mapping: mapping,
		handler: handler
	};
	return subId;
};

debugMan.unsubscribe = function(subId) {
	if ( ! this.subscriptions.hasOwnProperty(subId) ) return false;
	var keys = this.subscriptions[subId].keys;
	_.each(keys, _.bind(this._unsubscribeKey, this));
	delete this.subscriptions[subId];
	return true;
};

debugMan.subscribeImage = function(key, handler){
	console.log('subscribe image', key);
	this._subscribeKey([key]);

	var subId = _.uniqueId('sub');
	this.subscriptions[subId] =
	{
		isImage: true,
		key: key,
		keys: [[key]],
		handler: handler
	};
	return subId;
};
