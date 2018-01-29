var utils = require('util');
var EventEmitter = require('events').EventEmitter;
var net = require('net');

var CONFIG_PORT = 12442;
var HEADER_SIZE = 8;

var CM_SET = 0;
var CM_GET_MOUNTS = 1;
var CM_GET_KEYS = 2;
var CM_SAVE = 3;
var CM_SEND_KEYS = 4;
var CM_SEND_MOUNTS = 5;


function ConfigConnection() {
	if ( ! this instanceof ConfigConnection ) {
		throw new Exception('EEBControl is a constructor and needs to be called with keyword "new"');
	}
	EventEmitter.call(this);
};
utils.inherits(ConfigConnection, EventEmitter);

ConfigConnection.connect = function(hostname) {
	var instance = new ConfigConnection();
	instance.connect(hostname);
	return instance;
};

ConfigConnection.connectLocal = function(path) {
	var instance = new ConfigConnection();
	instance.connectLocal(path);
	return instance;
};

ConfigConnection.fromSocket = function(socket) {
	var instance = new ConfigConnection();
	instance.client = socket;
	instance.init();
	return instance;
};

ConfigConnection.prototype.connect = function(hostname) {
	if ( this.client ) {
		return this.emit('error', new Error('Already connected'));
	}
	this.hostname = hostname;
	this.client = net.connect({
		host: hostname,
		port: CONFIG_PORT
	});
	this.init();
};

ConfigConnection.prototype.connectLocal = function(path) {
	if ( this.client ) {
		return this.emit('error', new Error('Already connected'));
	}
	this.client = net.connect({
		path: path
	});
	this.init();
};

ConfigConnection.prototype.init = function() {
	this.headerBuffer = new Buffer(0);
	this.bodyBuffer = new Buffer(0);
	this.readHeader = true;
	this.receiveLength = HEADER_SIZE;

	var __self = this;
	this.client.on('connect', function() {
		__self.emit('connect');
	});
	this.client.on('error', function() {
		__self.emit('error', new Error('Connection error'));
	});
	this.client.on('timeout', function() {
		__self.emit('error', new Error('Connection timed out'));
	});
	this.client.on('close', function() {
		console.log('DebugProtocol.js: Peer disconnected');
		__self.emit('disconnect');
		__self.client = null;
	});
	this.client.on('data', this.onData.bind(this));
};

ConfigConnection.prototype.disconnect = function() {
	if ( this.client )
		this.client.end();
};

ConfigConnection.prototype.onData = function(data) {
	var lengthToParse = Math.min(this.receiveLength, data.length);
	if ( this.readHeader ) {
		this.headerBuffer = Buffer.concat( [ this.headerBuffer, data.slice(0, lengthToParse) ] );
	} else {
		this.bodyBuffer = Buffer.concat( [ this.bodyBuffer, data.slice(0, lengthToParse) ] );
	}
	this.receiveLength -= lengthToParse;

	if ( this.receiveLength == 0 && this.readHeader ) {
		this.readHeader = false;
		this.bodyBuffer = new Buffer(0);
		this.receiveLength = this.headerBuffer.readUInt16LE(6);
		var header = this.headerBuffer.toString('UTF-8',0,4);
		if ( header !== 'CONF' ) {
			console.error('received invalid config header:', header);
			this.emit('error', new Error('invalid config header received, disconnecting'));
			this.disconnect();
			return;
		}
	}
	if ( this.receiveLength == 0 && ! this.readHeader ) {
		var type = this.headerBuffer.readUInt8(5);
		// packet complete
		switch (type) {
			case CM_SET:
				this.emit('set', JSON.parse(this.bodyBuffer.toString('UTF-8')));
				break;
			case CM_GET_KEYS:
				this.emit('getKeys', this.bodyBuffer.toString('UTF-8'));
				break;
			case CM_GET_MOUNTS:
				this.emit('getMounts');
				break;
			case CM_SAVE:
				this.emit('save');
				break;
			case CM_SEND_KEYS:
				this.emit('sendKeys', JSON.parse(this.bodyBuffer.toString('utf8')));
				break;
			case CM_SEND_MOUNTS:
				this.emit('sendMounts', JSON.parse(this.bodyBuffer.toString('utf8')));
				break;
			default:
				this.emit('error', new Error('unknown message received'));
		}
		this.readHeader = true;
		this.headerBuffer = new Buffer(0);
		this.receiveLength = HEADER_SIZE;
	}

	data = data.slice(lengthToParse);
	if (data.length) {
		this.onData(data);
	}
};

ConfigConnection.prototype.sendDbgMessage = function(type, body) {
	if ( ! this.client ) {
		return this.emit('error', new Error('Not connected'));
	}
	var buf = new Buffer(HEADER_SIZE + body.length);
	buf.write('CONF', 0, 4, 'UTF-8');
	buf.writeUInt8(1, 4);
	buf.writeUInt8(type, 5);
	buf.writeUInt16LE(body.length, 6);
	buf.write(body, HEADER_SIZE);
	this.client.write(buf);
};

ConfigConnection.prototype.set = function(data){
	this.sendDbgMessage(CM_SET, JSON.stringify(data));
};

ConfigConnection.prototype.getMounts = function(){
	this.sendDbgMessage(CM_GET_MOUNTS, '');
};

ConfigConnection.prototype.getKeys = function(mountpoint){
	this.sendDbgMessage(CM_GET_KEYS, mountpoint);
};

ConfigConnection.prototype.save = function(){
	this.sendDbgMessage(CM_SAVE, '');
};

ConfigConnection.prototype.sendMounts = function(data){
	this.sendDbgMessage(CM_SEND_MOUNTS, JSON.stringify(data));
};

ConfigConnection.prototype.sendKeys = function(data){
	this.sendDbgMessage(CM_SEND_KEYS, JSON.stringify(data));
};

module.exports = ConfigConnection;
