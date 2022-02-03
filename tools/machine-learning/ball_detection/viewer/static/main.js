const init = async () => {
  await Promise.all([loadConfig(), loadSamples()]);
  setupGrid();
};

window.onload = init;
