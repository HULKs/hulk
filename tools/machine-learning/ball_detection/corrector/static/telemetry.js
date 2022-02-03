const telemetryId = (() => {
  const data = new Uint8Array(16);
  crypto.getRandomValues(data);
  return data.reduce((str, byte) => str + byte.toString(16).padStart(2, '0'), '');
})();

let telemetryMessageQueue = [];
let telemetryTimeout = null;

const sendTelemetryMessages = async messages => {
  try {
    await fetch(`/telemetry/${telemetryId}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(messages),
    });
  } catch (error) {
    console.error(error);
  }
};

const addTelemetryMessage = message => {
  if (telemetryTimeout === null) {
    sendTelemetryMessages([message]);

    telemetryTimeout = setTimeout(() => {
      telemetryTimeout = null;

      if (telemetryMessageQueue.length > 0) {
        sendTelemetryMessages(telemetryMessageQueue);
        telemetryMessageQueue = [];
      }
    }, 1000);
  } else {
    telemetryMessageQueue.push(message);
  }
};
