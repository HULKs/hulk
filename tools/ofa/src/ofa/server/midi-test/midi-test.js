var NK = require('../naoKONTROL2');
var ctrl = new NK();

ctrl.on('BTN_PUSH', function (btn) {
    ctrl.ledOn(btn);
});
ctrl.on('BTN_RELEASE', function (btn) {
    ctrl.ledOff(btn);
});
ctrl.on('REC_PUSH', function () {
    ctrl.ledOn(NK.BUTTON.PLAY);
});
ctrl.on('REC_RELEASE', function () {
    ctrl.ledOff(NK.BUTTON.PLAY);
});

ctrl.on('value', function (ch, val) {
    if (val == 0) console.log('channel', ch, 'at min');
    if (val == 127) console.log('channel', ch, 'at max');
});
