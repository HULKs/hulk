const PanelManager = require('./panelManager').manager;
export const panelManager = new PanelManager();

// === CONFIG ===
export const panels = {
  "Graph": require('./graph'),
  "Histogram2D": require('./histogram2d'),
  "xyGraph": require('./xyGraph'),
  "Queue": require('./queue'),
  "Export": require('./csvexport'),
  "Image": require('./showImage'),
  "CalibrationPoints": require('./calibrationPoints'),
  "Calibration Editor": require('./calibrationEditor'),
  "Map": require('./map'),
  "Relative": require('./relative'),
  "Config Editor": require('./configEditor')
};
