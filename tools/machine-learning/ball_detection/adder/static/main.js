const init = async () => {
  await Promise.all([loadConfig(), loadAnnotations()]);
  setupGrid();
  setupMaximize();

  addTelemetryMessage({
    'timestamp': (new Date).toISOString(),
    'type': 'pageVisibility',
    'visible': true,
  });
  window.addEventListener('visibilitychange', () => {
    addTelemetryMessage({
      'timestamp': (new Date).toISOString(),
      'type': 'pageVisibility',
      'visible': document.hidden,
    });
  });

  addTelemetryMessage({
    'timestamp': (new Date).toISOString(),
    'type': 'windowSize',
    'width': window.innerWidth,
    'height': window.innerHeight,
    'zoom': window.devicePixelRatio,
  });
  window.addEventListener('resize', () => {
    addTelemetryMessage({
      'timestamp': (new Date).toISOString(),
      'type': 'windowSize',
      'width': window.innerWidth,
      'height': window.innerHeight,
      'zoom': window.devicePixelRatio,
    });
  });
};

window.onload = init;
