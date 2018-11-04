/*var net = require('net');

var CM_SET = 0;
var CM_GET = 1;
var CM_SAVE = 2;

var client = net.connect({
	host:'tuhhnao15.lan',
	port: 12442
});

function sendMsg(type, body) {
	var buf = new Buffer(8 + body.length);
	buf.write('CONF', 0, 4, 'UTF-8');
	buf.writeUInt8(1, 4);
	buf.writeUInt8(type, 5);
	buf.writeUInt16LE(body.length, 6);
	buf.write(body, 8);
	client.write(buf);
};
client.on('connect', function() {
	console.log('connected to nao');
	sendMsg(CM_SET, JSON.stringify([ { mp:"camSettings", key:"top.gain", value:42 } ]));
	sendMsg(CM_SET, JSON.stringify([ { mp:"camSettings", key:"bottom.gain", value:65 } ]));
	sendMsg(CM_SAVE,'');
});
*/
var Config = require('./ConfigProtocol.js');

var client = Config.connect('tuhhnao13.lan');

client.on('connect', function () {
    client.set([
        {mp: "motion", key: "pid", value: {"p": 42.5}}
    ]);
    client.save();
});
