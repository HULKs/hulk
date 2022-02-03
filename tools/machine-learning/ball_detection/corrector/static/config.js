let config = {};

const loadConfig = async () => {
  const response = await fetch(`/config.json`);
  config = await response.json();
};
