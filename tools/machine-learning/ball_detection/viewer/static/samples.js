let amountOfSamples = 0;

const loadSamples = async () => {
  const samplesResponse = await fetch('/samples.json');
  amountOfSamples = await samplesResponse.json();
};
