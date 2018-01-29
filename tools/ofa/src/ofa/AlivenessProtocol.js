var utils = require('util');
var EventEmitter = require('events').EventEmitter;
var dgram = require('dgram');

var ALIVENESS_PORT = 12440;
var MESSAGE_SIZE = 68;
var HEADER_MAGIC = 'LIVE';
var MAX_ROBOT_AGE = 2000;

function AlivenessListener() {
	if (!(this instanceof AlivenessListener)) {
		throw new Exception('AlivenessListener is a constructor and needs to be called with keyword "new"');
	}
	EventEmitter.call(this);
}
utils.inherits(AlivenessListener, EventEmitter);

AlivenessListener.prototype.listen = function() {
	if (this.client) {
		return this.emit('error', new Error('Already listening'));
	}
	this.client = dgram.createSocket({
		type: 'udp4',
		reuseAddr: true
	});
	this.client.bind(ALIVENESS_PORT);
	this.client.on('message', this.onMessage.bind(this));
	this.client.on('error', this.onError.bind(this));
	this.alive = { };
	this.agingTimer = setInterval(this.onTimer.bind(this), MAX_ROBOT_AGE);
};

AlivenessListener.prototype.getAlive = function() {
	return this.alive;
};

AlivenessListener.prototype.onMessage = function(message, remote) {
	if (message.length != MESSAGE_SIZE) {
		throw new Error('Received invalid aliveness message, wrong size');
	}
	var changed = null;
	var header = message.toString('utf-8', 0, 4);
	if (header !== HEADER_MAGIC) {
		throw new Error('Received invalid aliveness message, wrong magic value');
	}
	var body = message.toString('ascii', 4, 36).split('\0')[0];
	var head = message.toString('ascii', 36, 68).split('\0')[0];
	if (!this.alive.hasOwnProperty[remote.address]) {
		changed = remote.address;
	}
	this.alive[remote.address] = { head: head, body: body, timestamp: (new Date).getTime() };
	if (changed) {
		this.emit('changed', this.alive, remote.address);
	}
};

AlivenessListener.prototype.onError = function() {
	this.emit('error', new Error('Connection error'));
};

AlivenessListener.prototype.onTimer = function() {
	var now = (new Date).getTime();
	var changed = false;
	for (var addr in this.alive) {
		if ((now - this.alive[addr].timestamp) >= MAX_ROBOT_AGE) {
			changed = true;
			delete this.alive[addr];
		}
	}
	if (changed) {
		this.emit('changed', this.alive, addr);
	}
};

module.exports = AlivenessListener;
