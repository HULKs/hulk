// === CONFIG ===
var panels = {
    "Graph": Graph,
    "Histogram2D": Histogram2d,
    "xyGraph": XYGraph,
    "Queue": Queue,
    "Export": CSVExport,
    "Image" : ImageStream,
    "CalibrationPoints": CalibrationPoints,
    "Map": Map,
    "Relative": RelativeMap,
    "Config Editor": ConfigEditor
};

var naos = {
    '10.0.24.21' : 'tuhhnao11 (WLAN)',
    '10.1.24.21' : 'tuhhnao11 (LAN)',
    '10.0.24.22' : 'tuhhnao12 (WLAN)',
    '10.1.24.22' : 'tuhhnao12 (LAN)',
    '10.0.24.23' : 'tuhhnao13 (WLAN)',
    '10.1.24.23' : 'tuhhnao13 (LAN)',
    '10.0.24.24' : 'tuhhnao14 (WLAN)',
    '10.1.24.24' : 'tuhhnao14 (LAN)',
    '10.0.24.25' : 'tuhhnao15 (WLAN)',
    '10.1.24.25' : 'tuhhnao15 (LAN)',
    '10.0.24.26' : 'tuhhnao16 (WLAN)',
    '10.1.24.26' : 'tuhhnao16 (LAN)',
    '10.0.24.27' : 'tuhhnao17 (WLAN)',
    '10.1.24.27' : 'tuhhnao17 (LAN)'
}

// === INIT ===
var panelManager = new PanelManager();
var navigation = new Navigation();
var viewManager = new ViewManager(ioSocket, navigation, panelManager);
debugMan.init();
connectionManager.init(naos, location.hash ? location.hash.substr(1) : null);
