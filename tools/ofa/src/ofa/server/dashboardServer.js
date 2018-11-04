var _ = require('underscore');
var express = require('express');
var app = express();
var httpServer = require('http').Server(app);
var io = require('socket.io')(httpServer);
var fs = require('fs');
var path = require('path');

var DebugConnection = require('./DebugProtocol.js');
var AlivenessListener = require('./AlivenessProtocol.js');

var naoStatus = {};

// Configuration
var httpPort = 8100;
var execDir = require.main ? path.dirname(require.main.filename) : process.cwd();


// === load last slider config ===
var naoStatusFileName = execDir + '/naoStatus.json';
if (fs.existsSync(naoStatusFileName)) {
    var content = fs.readFileSync(naoStatusFileName, 'utf8');
    naoStatus = JSON.parse(content);
    _.each(naoStatus, function (nao, headName) {
        nao.head = headName;
        nao.ip = '';
        nao.batteryLevel = null;
        nao.lastAliveness = null
        nao.debugConn = null;
    });
    console.log('loaded NAO status from', naoStatusFileName);
}


// === init AlivenessListener ===
var aliveness = new AlivenessListener();
aliveness.listen();
aliveness.on('changed', function (alive, changedIP) {
    if (!alive.hasOwnProperty(changedIP)) {
        var nao = _.findWhere(naoStatus, {ip: changedIP});
        if (nao) setOffline(nao);
        return;
    }
    var changedData = alive[changedIP];
    if (!naoStatus.hasOwnProperty(changedData.head)) return;

    var nao = naoStatus[changedData.head];
    nao.lastAliveness = Date.now();
    if (!nao.ip) {
        nao.ip = changedIP;
        initDebugConnection(nao);
    }
    nao.body = changedData.body;
    io.emit('updateStatus', _.omit(nao, 'debugConn'));
});

function setOffline(nao) {
    nao.ip = '';
    nao.batteryLevel = null;
    nao.lastAliveness = null;
    nao.debugConn = null;
    io.emit('updateStatus', formatInfo(nao));
}

function formatInfo(nao) {
    return _.pick(nao, 'ip', 'head', 'body', 'batteryLevel', 'comment');
}

function initDebugConnection(nao) {
    nao.debugConn = new DebugConnection();
    nao.debugConn.connect(nao.ip);
    nao.debugConn.once('connect', function () {
        nao.debugConn.subscribeBulk(['tuhhSDK.batteryDisplay.smoothedBatteryCharge']);
        nao.debugConn.on('update', _.bind(parseDebugUpdate, nao));
    });
    nao.debugConn.on('error', function (e) {
        console.log('debug connection error (' + nao.head + '): ' + e.message);
    });
    nao.debugConn.once('disconnect', function () {
        setOffline(nao);
    });
}

function parseDebugUpdate(data) {
    var info = _.indexBy(data, 'key');
    this.batteryLevel = info['tuhhSDK.batteryDisplay.smoothedBatteryCharge'].value;
}

function saveComment(headName, text) {
    naoStatus[headName].comment = text;
    io.emit('updateComment', headName, text);
}


// === IO.Socket Connection handling ===
// cconstruct debug and config connections
// and create all event redirection
io.on('connection', function (socket) {
    socket.emit('init', _.map(naoStatus, formatInfo));
    socket.emit('setVersion', 1);
    socket.on('saveComment', saveComment);
});


// === Basic HTTP server initialization ===
app.use('/style', express.static(__dirname + '/../../style'));
app.use('/libs', express.static(__dirname + '/../../libs'));
app.use(express.static(__dirname + '/dashboard'));

app.get('/', function (req, res) {
    res.redirect('index.htm');
});


httpServer.listen(httpPort, function () {
    console.log('http server listening on *:' + httpPort);
});
