var EventEmitter = require('events').EventEmitter;
var util = require('util');

var midiAvailable = false;
var midi = null;
try {
	midi = require('midi');
	midiAvailable = true;
} catch (e) {
	console.log('midi package not installed, naoKontrol will not be available!');
}


function naoKontrol() {
	EventEmitter.call(this);
	this.init.apply(this, arguments);
}
util.inherits(naoKontrol, EventEmitter);
module.exports = naoKontrol;

naoKontrol.prototype.init = function(channel) {
	this.portNum = null;
	if (!midiAvailable) return;
	this.input = new midi.input();
	this.output = new midi.output();
	this.portNum = this.findPort();
	this.channel = channel || 0;

	this.input.on('message', this.messageHandler.bind(this));
};

naoKontrol.prototype.found = function() {
	return this.portNum !== null;
};

naoKontrol.prototype.openPort = function() {
	if ( ! this.found() ) {
		throw new Error('nanoKONTROL2 device not found');
	}
	this.input.openPort(this.portNum);
	this.output.openPort(this.portNum);
};

naoKontrol.prototype.findPort = function() {
	if (!midiAvailable) return null;
	var numPorts = this.input.getPortCount(),
		portName, nameCheck = /nanokontrol2/i;
	for ( var i = 0; i < numPorts; i++ ) {
		portName = this.input.getPortName(i);
		if ( nameCheck.test(portName) ) {
			return i;
		}
	}
	return null;
};

naoKontrol.prototype.messageHandler = function(time, data) {
	// 0xB# is the first byte of Control Change messages,
	// with # being the number of the global channel.
	// Here we drop other messages.
	if ( data[0] != (0xB0 | this.channel) ) return;

	var channel = data[1],
		value = data[2];
	if ( __valueIds.indexOf(channel) >= 0 )
		this.emit('value', channel, value);
	if ( __buttonReverseMap.hasOwnProperty(channel) ) {
		var state = value == 127 ? 'PUSH' : 'RELEASE';
		this.emit('BTN_'+state, channel);
		this.emit(__buttonReverseMap[channel]+'_'+state);
	}
};

naoKontrol.prototype.ledOn = function(id) {
	if (!midiAvailable) return;
	this.output.sendMessage([0xB0|this.channel, id, 127]);
};

naoKontrol.prototype.ledOff = function(id) {
	if (!midiAvailable) return;
	this.output.sendMessage([0xB0|this.channel, id, 0]);
};


// === CONSTANTS ===
naoKontrol.SLIDER = [ 0, 1, 2, 3, 4, 5, 6, 7];
naoKontrol.KNOB   = [16,17,18,19,20,21,22,23];
naoKontrol.BUTTON = {
	PLAY:	41,
	STOP:	42,
	RWND:	43,
	FFWD:	44,
	REC:	45,
	CYCLE:	46,
	TRACK_PREV:	58,
	TRACK_NEXT:	59,
	MARKER_SET:	60,
	MARKER_PREV:	61,
	MARKER_NEXT:	62,
};
for ( var i = 0; i < 8; i++ ) {
	naoKontrol.BUTTON['SLIDER_SOLO_'+i] = i + 32;
	naoKontrol.BUTTON['SLIDER_MUTE_'+i] = i + 48;
	naoKontrol.BUTTON['SLIDER_REC_' +i] = i + 64;
}
// === /CONSTANTS ===

// === helper vars ===
var __valueIds = naoKontrol.SLIDER.concat(naoKontrol.KNOB);
var __buttonReverseMap = {};
for ( var key in naoKontrol.BUTTON ) {
	if ( !naoKontrol.BUTTON.hasOwnProperty(key) ) continue;
	var val = naoKontrol.BUTTON[key];
	__buttonReverseMap[val] = key;
}
// === /helper vars ===
