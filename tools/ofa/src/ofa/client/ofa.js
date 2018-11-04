var Navigation = require('./ui').Navigation;
var ViewManager = require('./viewManager');
var conn = require('./connection');
var panels = require('./globals').panels;

var ioSocket = conn.ioSocket;
var debugMan = conn.debugMan;
var connectionManager = conn.connectionManager;

var naos = {
    '10.0.24.21': 'tuhhnao11 (WLAN)',
    '10.1.24.21': 'tuhhnao11 (LAN)',
    '10.0.24.22': 'tuhhnao12 (WLAN)',
    '10.1.24.22': 'tuhhnao12 (LAN)',
    '10.0.24.23': 'tuhhnao13 (WLAN)',
    '10.1.24.23': 'tuhhnao13 (LAN)',
    '10.0.24.24': 'tuhhnao14 (WLAN)',
    '10.1.24.24': 'tuhhnao14 (LAN)',
    '10.0.24.25': 'tuhhnao15 (WLAN)',
    '10.1.24.25': 'tuhhnao15 (LAN)',
    '10.0.24.26': 'tuhhnao16 (WLAN)',
    '10.1.24.26': 'tuhhnao16 (LAN)',
    '10.0.24.27': 'tuhhnao17 (WLAN)',
    '10.1.24.27': 'tuhhnao17 (LAN)'
};

// === INIT ===
module.exports = {
    init: function () {
        var navigation = new Navigation(panels);
        var viewManager = new ViewManager(ioSocket, navigation);
        debugMan.init();
        connectionManager.init(naos, location.hash ? location.hash.substr(1) : null);
    }
};

