Debug = require('./DebugProtocol.js');

debugClient = new Debug();

debugClient.connect('tuhhnao12.lan');
debugClient.listCommands();
//debugClient.sendSubscribe('TestKey');
debugClient.subscribeBulk(['TestKey', 'tuhhSDK.Blackboard.Fsr.Left', 'tuhhSDK.Blackboard.Fsr.Right']);
debugClient.on('list', function(data) {
	console.log('received list:', data);
});
debugClient.on('update', function(data) {
	console.log('received update:', data);
});

setTimeout(function() {
	debugClient.unsubscribe('TestKey');
}, 2000);
