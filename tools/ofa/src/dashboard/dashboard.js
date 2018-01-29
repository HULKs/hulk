var ioSocket = io();
var version = 0;
var naos = {};
var dashboardContainer = $('.naos');
ioSocket.on('init', function(naoStatus) {
	//console.log('init', naoStatus);
	dashboardContainer.empty();
	_.each(naoStatus, function(status) {
		var nao = { status: status, elem:{} };
		nao.elem.container = $('<div>');
		nao.elem.headline = $('<h2>').text(status.head).appendTo(nao.elem.container);
		nao.elem.bodyName = $('<span class="bodyName">').appendTo(nao.elem.headline);

		nao.elem.body = $('<div class="infos">').appendTo(nao.elem.container);
		nao.elem.ipAddress = $('<div class="ipAddress">').appendTo(nao.elem.body);
		nao.elem.battery = $('<div class="battery">').text('--').appendTo(nao.elem.body);
		nao.elem.commentBox = $('<textarea>').val(status.comment).appendTo(nao.elem.body);
		nao.elem.commentBox.blur(_.bind(saveCommentBox, nao));

		dashboardContainer.append(nao.elem.container);
		naos[status.head] = nao;
	})
});
ioSocket.on('updateStatus', function(update) {
	if ( ! naos.hasOwnProperty(update.head) ) return;
	var nao = naos[update.head];
	nao.status = update;
	nao.elem.bodyName.text(update.body);
	nao.elem.ipAddress.text(update.ip);
	nao.elem.battery.text(update.batteryLevel == null ? '--' : Math.round(update.batteryLevel*100));
	if ( update.ip )
		nao.elem.container.addClass('online');
	else
		nao.elem.container.removeClass('online');
});

function saveCommentBox() {
	var text = this.elem.commentBox.val();
	ioSocket.emit('saveComment', this.status.head, text);
}

ioSocket.on('updateComment', function(headName, text) {
	if ( ! naos.hasOwnProperty(headName) ) return;
	var nao = naos[headName];
	nao.status.comment = text;
	nao.elem.commentBox.val(text);
});

ioSocket.on('setVersion', function(newVersion) {
	if ( !version ) {
		version = newVersion;
		return;
	}
	if ( version < newVersion ) {
		location.reload();
	}
});
