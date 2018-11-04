var DebugConnection = require('./DebugProtocol.js');
var ConfigConnection = require('./ConfigProtocol.js');
var AlivenessListener = require('./AlivenessProtocol.js');
var naoKontrol = require('./naoKONTROL2.js');
var ViewStorage = require('./ViewStorage.js');
var _ = require('underscore');
var express = require('express');
var app = express();
var httpServer = require('http').Server(app);
var io = require('socket.io')(httpServer);
var fs = require('fs');
var path = require('path');
var colormap = require('colormap');
var rainbow = colormap({colormap: 'rainbow', nshades: 256, format: 'rgb'});
var portland = colormap({colormap: 'portland', nshades: 256, format: 'rgb'});

var sliderMap = {};
var latestConfigConnection = null;
var latestSocketConnection = null;

// in-memory storage for ofa images.
var imageBuffers = {};

// Configuration - http port so far
var httpPort = 8000;

// === load last slider config ===
var execDir = require.main ? path.dirname(require.main.filename) : process.cwd();
var sliderMapFileName = execDir + '/slider.json';
if (fs.existsSync(sliderMapFileName)) {
    var content = fs.readFileSync(sliderMapFileName, 'utf8');
    sliderMap = JSON.parse(content);
    console.log('loaded slider config from', sliderMapFileName);
}

// === init ViewStorage ===
var views = new ViewStorage(io);

// === init AlivenessListener ===
var aliveness = new AlivenessListener();
aliveness.listen();
aliveness.on('changed', function (alive) {
    io.emit('alive', alive);
});

let nextConnectionId = 0;

// === IO.Socket Connection handling ===
// cconstruct debug and config connections
// and create all event redirection
io.on('connection', function (socket) {
    const connectionId = nextConnectionId++;
    latestSocketConnection = socket;
    socket.emit('colormap_rainbow', rainbow);
    socket.emit('colormap_portland', portland);
    socket.emit('naoKontrolAvailable', ctrl.found());
    socket.emit('alive', aliveness.getAlive());

    var debug = new DebugConnection();
    debug.on('connect', function () {
        console.log('debugConnect');
        socket.emit('debugConnect', connectionId);
    });
    debug.on('error', function (err) {
        console.log('debugError', err.message);
        socket.emit('debugError', err.message);
    });
    debug.on('disconnect', function () {
        console.log('debugDisconnect');
        socket.emit('debugDisconnect');
    });
    debug.on('update', function (data) {
        socket.emit('update', data);
    });
    debug.on('list', function (list) {
        socket.emit('list', list);
    });
    debug.on('image', function (imageKey, image) {
        imageBuffers[connectionId + '-' + imageKey] = image;
        socket.emit('image', imageKey);
    });

    var config = latestConfigConnection = new ConfigConnection();
    config.on('connect', function () {
        console.log('configConnect');
        socket.emit('configConnect');
    });
    config.on('error', function (err) {
        console.log('configError', err.message);
        socket.emit('configError', err.message);
    });
    config.on('disconnect', function () {
        console.log('configDisconnect');
        socket.emit('configDisconnect');
        latestConfigConnection = null;
    });
    config.on('sendMounts', function (data) {
        console.log('config.sendMounts', data);
        socket.emit('config.sendMounts', data);
    });
    config.on('sendKeys', function (data) {
        console.log('config.sendKeys', data);
        socket.emit('config.sendKeys', data);
    });


    socket.on('disconnect', function () {
        console.log('disconnect');
        debug.disconnect();
        config.disconnect();
        latestConfigConnection = null;
        latestSocketConnection = null;
    });
    socket.on('disconnectNao', function () {
        console.log('disconnectNao');
        debug.disconnect();
        config.disconnect();
        latestConfigConnection = null;
        latestSocketConnection = null;
    });
    socket.on('connectNao', function (hostname) {
        console.log('connectNao', hostname);
        debug.connect(hostname);
        config.connect(hostname);
    });
    socket.on('connectLocal', function (path) {
        console.log('connectLocal', path);
        debug.connectLocal(path + "/debug");
        config.connectLocal(path + "/config");
    });
    socket.on('debug.subscribe', function (key) {
        console.log('debug.subscribe', key);
        debug.subscribe(key);
    });
    socket.on('debug.subscribeBulk', function (keys) {
        console.log('debug.subscribeBulk', keys);
        debug.subscribeBulk(keys);
    });
    socket.on('debug.unsubscribe', function (key) {
        console.log('debug.unsubscribe', key);
        debug.unsubscribe(key);
    });
    socket.on('debug.listCommands', function () {
        console.log('debug.listCommands');
        debug.listCommands();
    });

    socket.on('config.set', function (data) {
        console.log('config.set', data);
        config.set(data);
    });
    socket.on('config.getMounts', function () {
        console.log('config.getMount');
        config.getMounts();
    });
    socket.on('config.getKeys', function (mountpoint) {
        console.log('config.getKeys', mountpoint);
        config.getKeys(mountpoint);
    });
    socket.on('config.save', function () {
        console.log('config.save');
        config.save();
    });
    socket.on('config.map', function (cfg) {
        console.log('config.map', cfg);
        sliderMap[cfg.ch] = cfg;
        // update slider config file
        fs.writeFile(sliderMapFileName, JSON.stringify(sliderMap, null, '\t'), {encoding: 'utf8'}, function () {
            console.log('Saved slider config to', sliderMapFileName);
        });
    });
});


// === Basic HTTP server initialization ===
app.use(express.static(__dirname + '/public'));
app.use("/style", express.static(__dirname + '/../../style'));
app.use("/libs", express.static(__dirname + '/../../libs'));

app.get(/^\/$|^\/view\//, function (req, res) {
    res.sendFile(path.join(__dirname, 'public', 'index.html'));
});

// Image delivery
app.get('/image/:imageKey', function (req, res) {
    var key = req.params.imageKey;
    if (!imageBuffers.hasOwnProperty(key)) {
        return res.sendStatus(404);
    }
    res.set('Content-Type', 'image/jpeg');
    res.send(imageBuffers[key]);
});

httpServer.listen(httpPort, function () {
    console.log('http server listening on *:' + httpPort);
});


var ctrl = new naoKontrol();
if (!ctrl.found()) {
    console.log('WARNING: naoKontrol not available');
} else {
    ctrl.openPort();
}

ctrl.on('value', function (ch, val) {
    if (latestConfigConnection === null || !sliderMap.hasOwnProperty(ch)) {
        return;
    }
    var cfg = sliderMap[ch];
    var mappedVal = val / 127 * (cfg.max - cfg.min) + cfg.min;
    setConfig([{mp: cfg.mp, key: cfg.key, value: mappedVal}]);
});
var setConfig = _.debounce(function (data) {
    latestConfigConnection.set(data);
    latestSocketConnection.emit('config.set', data);
}, 200);
ctrl.on('REC_PUSH', function () {
    if (latestConfigConnection === null) {
        return;
    }
    latestConfigConnection.save();
});
