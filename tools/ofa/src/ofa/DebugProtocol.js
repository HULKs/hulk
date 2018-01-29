var utils = require('util');
var EventEmitter = require('events').EventEmitter;
var net = require('net');
var file = require('fs');


var DEBUG_PORT = 12441;
var HEADER_SIZE = 16;

var DM_SUBSCRIBE = 0;
var DM_UNSUBSCRIBE = 1;
var DM_UPDATE = 2;
var DM_REQUEST_LIST = 3;
var DM_LIST = 4;
var DM_SUBSCRIBE_BULK = 5;
var DM_IMAGE = 6;


function DebugConnection() {
	if ( ! this instanceof DebugConnection ) {
		throw new Exception('DebugConnection is a constructor and needs to be called with keyword "new"');
	}
	EventEmitter.call(this);
}

utils.inherits(DebugConnection, EventEmitter);

DebugConnection.connect = function(hostname) {
	var instance = new DebugConnection();
	instance.connect(hostname);
	return instance;
};

DebugConnection.connectLocal = function(path) {
	var instance = new DebugConnection();
	instance.connectLocal(path);
	return instance;
};

DebugConnection.fromSocket = function(socket) {
	var instance = new DebugConnection();
	instance.client = socket;
	instance.init();
	return instance;
};

DebugConnection.prototype.connect = function(hostname) {
	if ( this.client ) {
		return this.emit('error', new Error('Already connected'));
	}
	this.client = net.connect({
		host: hostname,
		port: DEBUG_PORT
	});
	this.init();
};

DebugConnection.prototype.connectLocal = function(path) {
	if ( this.client ) {
		return this.emit('error', new Error('Already connected'));
	}
	this.client = net.connect({
		path: path
	});
	this.init();
};

DebugConnection.prototype.init = function() {
	this.headerBuffer = new Buffer(0);
	this.bodyBuffer = new Buffer(0);
	this.readHeader = true;
	this.receiveLength = HEADER_SIZE;

	var __self = this;
	this.client.on('connect', function() {
		__self.emit('connect');
	});
	this.client.on('error', function(e) {
		console.log('Client connect error', e);
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

DebugConnection.prototype.disconnect = function() {
	if ( this.client )
		this.client.end();
};

DebugConnection.prototype.convertImage = function (bodyBuffer) {
	var width = bodyBuffer.readUInt16LE(0);
	var height = bodyBuffer.readUInt16LE(2);
	var keyLength = bodyBuffer.readUInt16LE(4);
	var imageKey = bodyBuffer.toString('utf8', 6, 6 + keyLength);
	var image = bodyBuffer.slice(6+keyLength);

	this.emit('image', imageKey, image);
};


DebugConnection.prototype.onData = function(data) {
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
		this.receiveLength = this.headerBuffer.readUInt32LE(8);
		//console.log('received header, expected body length:', this.receiveLength);
		var header = this.headerBuffer.toString('UTF-8',0,4);
		if ( header !== 'DMSG' ) {
			console.error('received invalid debug header:', header);
			this.emit('error', new Error('invalid debug header received, disconnecting'));
			this.disconnect();
			return;
		}
	}
	if ( this.receiveLength == 0 && ! this.readHeader ) {
		//console.log('received body staring with ', this.bodyBuffer.toString('UTF-8', 0, 2));
		var type = this.headerBuffer.readUInt8(5);
		try {
			// packet complete
			switch (type) {
				case DM_SUBSCRIBE:
					this.emit('subscribe', this.bodyBuffer.toString('UTF-8'));
					break;
				case DM_UNSUBSCRIBE:
					this.emit('unsubscribe', this.bodyBuffer.toString('UTF-8'));
					break;
				case DM_UPDATE:
					this.emit('update', JSON.parse(this.bodyBuffer.toString('UTF-8')));
					break;
				case DM_REQUEST_LIST:
					this.emit('requestList');
					break;
				case DM_LIST:
					this.emit('list', JSON.parse(this.bodyBuffer.toString('UTF-8')));
					break;
				case DM_SUBSCRIBE_BULK:
					this.emit('subscribeBulk', JSON.parse(this.bodyBuffer.toString('UTF-8')));
					break;
				case DM_IMAGE:
					//console.log('received image');
					this.convertImage(this.bodyBuffer);
					break;
				default:
					this.emit('error', new Error('unknown message received'));
			}
		} catch (e) {
			console.error('Error when parsing message: ' + e.message);
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

DebugConnection.prototype.sendDbgMessage = function(type, body) {
	if ( ! this.client ) {
		return this.emit('error', new Error('Not connected'));
	}
	var buf = new Buffer(HEADER_SIZE + body.length);
	buf.write('DMSG', 0, 4, 'UTF-8');
	buf.writeUInt8(1, 4);
	buf.writeUInt8(type, 5);
	buf.writeUInt32LE(body.length, 8);
	buf.write(body, HEADER_SIZE);
	this.client.write(buf);
};


DebugConnection.prototype.subscribe = function(data){
	console.log('Sending Subscribe for key '+data);
	this.sendDbgMessage(DM_SUBSCRIBE, data);
};

DebugConnection.prototype.unsubscribe = function(data){
	console.log('Sending Unsubscribe for key '+data);
	this.sendDbgMessage(DM_UNSUBSCRIBE, data);
};

DebugConnection.prototype.update = function(data){
	this.sendDbgMessage(DM_UPDATE, JSON.stringify(data));
};

DebugConnection.prototype.listCommands = function(){
	this.sendDbgMessage(DM_REQUEST_LIST, '');
};

DebugConnection.prototype.list = function(data){
	this.sendDbgMessage(DM_LIST, JSON.stringify(data));
};

DebugConnection.prototype.subscribeBulk = function(data){
	this.sendDbgMessage(DM_SUBSCRIBE_BULK, JSON.stringify({ keys: data }));
};

module.exports = DebugConnection;
