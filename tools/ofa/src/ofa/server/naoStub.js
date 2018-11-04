var Debug = require('./DebugProtocol.js');
var Config = require('./ConfigProtocol');
var net = require('net');
var dgram = require('dgram');
var _ = require('underscore');

var basePort = 12440;
var data = {};
var sessions = [];

var alivenessSender = dgram.createSocket('udp4');
var alivenessMsg = new Buffer(68);
alivenessMsg.fill(0);
alivenessMsg.write('LIVE', 0);
alivenessMsg.write('naoStubBody404', 4);
alivenessMsg.write('naoStub', 36);
setInterval(function () {
    alivenessSender.send(alivenessMsg, 0, alivenessMsg.length, basePort);
}, 1000);

var configServer = net.createServer(function (socket) {
    var connection = Config.fromSocket(socket);

    connection.on('set', function (data) {
        console.log('Config Set: ', data);
    });
    connection.on('getMounts', function () {
        connection.sendMounts({
            keys: [
                {key: 'tuhhSDK.base', filename: '/home/nao/naoqi/preferences/sdk/sdk.json'},
                {key: 'Brain.fieldColor', filename: '/home/nao/naoqi/preferences/brain/fieldColor.json'}
            ]
        });
        // Test code for config header error handling
        /*setTimeout(function() {
            var buf = new Buffer('ERR');
            socket.write(buf);
        }, 5000);*/
    });
    connection.on('getKeys', function (mountPoint) {
        if (mountPoint == 'tuhhSDK.base') {
            connection.sendKeys({
                keys: [{key: 'test', value: 50}, {key: 'test2', value: 100}],
                mountPoint: 'tuhhSDK.base'
            });
        }
        if (mountPoint == 'Brain.fieldColor') {
            connection.sendKeys({
                keys: [{key: 'color', value: 127}],
                mountPoint: 'Brain.fieldColor'
            });
        }
    });
});

configServer.listen(basePort + 2);

var server = net.createServer(function (socket) {
    var connection = Debug.fromSocket(socket);
    var session = {
        connection: connection,
        subscribedKeys: []
    };
    sessions.push(session);
    // Test code for debug header error handling
    /*setTimeout(function() {
        var buf = new Buffer('ERR');
        socket.write(buf);
    }, 5000);*/
    connection.on('requestList', function () {
        sendKeyList(session.connection);
    });
    connection.on('subscribe', function (key) {
        session.subscribedKeys.push(key);
    });
    connection.on('subscribeBulk', function (data) {
        session.subscribedKeys = _.uniq(session.subscribedKeys.concat(data.keys));
        console.log('subscribeBulk. now subscribed to:', session.subscribedKeys);
    });
    connection.on('unsubscribe', function (key) {
        session.subscribedKeys.splice(
            session.subscribedKeys.indexOf(key), 1);
    });
    connection.on('disconnect', function () {
        sessions.splice(sessions.indexOf(session), 1);
    });
});
server.listen(basePort + 1);

function sendKeyList(connection) {
    var keys = _.map(data, function (val) {
        return {
            key: val.key,
            type: val.type,
            isArray: val.isArray,
            arrayLength: val.isArray ? val.value.length : 0,
            value: val.value
        };
    });
    console.log('sending keyList:', keys);
    connection.list({keys: keys});
}

function update(key, value, type) {
    var isArray = _.isArray(value);
    data[key] = {
        key: key,
        timestamp: Math.round((new Date).getTime() / 1000),
        type: type,
        isArray: isArray,
        value: value
    };
}

function transport() {
    for (var i = 0; i < sessions.length; i++) {
        var session = sessions[i];
        var update = _.filter(data, function (value) {
            return _.contains(session.subscribedKeys, value.key);
        });
        if (update.length) {
            sessions[i].connection.update(update);
        }
    }
}


function Vector2(x, y) {
    this.val = [x, y]
}

Vector2.prototype.toJSON = function () {
    return this.val
};

function Vector3(x, y, z) {
    this.val = [x, y, z]
}

Vector3.prototype.toJSON = function () {
    return this.val
};


var TestKey = 0.5;
var FakeIMU = new Vector2(0, 0);
var FakeJoints = [0, 0.2, 0.1, 0.2]

function cycle() {
    TestKey = TestKey + (Math.random() - 0.5) * 0.1;
    FakeIMU.val[0] = FakeIMU.val[0] + (Math.random() - 0.5) * 0.2;
    FakeIMU.val[1] = FakeIMU.val[1] + (Math.random() - 0.5) * 0.2;
    FakeJoints[0] = FakeJoints[0] + (Math.random() - 0.5) * 0.1;
    FakeJoints[1] = FakeJoints[1] + (Math.random() - 0.5) * 0.1;
    FakeJoints[2] = FakeJoints[2] + (Math.random() - 0.5) * 0.1;
    FakeJoints[3] = FakeJoints[3] + (Math.random() - 0.5) * 0.1;

    update('TestKey', TestKey, 'float');
    update('FakeIMU', FakeIMU, 'vector2');
    update('FakeJoints', FakeJoints, 'float');
    transport();
}

setInterval(cycle, 10);
