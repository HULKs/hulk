function ViewManager(ioSocket, navigation, panelManager) {
	this.ioSocket = ioSocket;
	this.navigation = navigation;
	this.panelManager = panelManager;
	this.views = [];
	this.currentView = null;

	this.hasChanges = false;
	this.panelManager.on('change', _.bind(this.onPanelsChange, this));

	this.viewDropdown = document.createElement('nav');
	this.viewDropdown.className = 'viewList';
	this.viewList = document.createElement('ul');
	this.viewDropdown.appendChild(this.viewList);
	ui.body.appendChild(this.viewDropdown);

	this.selectViewBtn = navigation.addNavItem('Open View ...', _.bind(this.onSelectViewClick, this));
	this.saveBtn = navigation.addNavItem('Save', _.bind(this.onSaveClick, this));
	this.newBtn = navigation.addNavItem('New', _.bind(this.onNewClick, this));

	ui.body.addEventListener('click', _.bind(this.onBodyClick, this));
	window.addEventListener('beforeunload', _.bind(this.onBeforeUnload, this));
	window.addEventListener('popstate', _.bind(this.onPopState, this));

	this.ioSocket.on('views.update', _.bind(this.onViewsUpdate, this));
}

ViewManager.prototype.onPanelsChange = function() {
	this.hasChanges = true;
}

ViewManager.prototype.onViewsUpdate = function(views) {
	this.views = [];
	this.currentView = null;
	this.viewList.innerHTML = '';

	for(var id in views){
		var view = {
			id: id,
			json: views[id]
		};
		this.addDropdownItem(view);
		this.views.push(view);
	}

	this.loadFromUrl();
}

ViewManager.prototype.loadFromUrl = function() {
	var viewUrl = location.pathname.match(/^\/view\/(.+)$/);
	if ( ! viewUrl ) return;
	var viewFromUrl = _.findWhere(this.views, { id: decodeURIComponent(viewUrl[1]) });
	if ( ! viewFromUrl ) return;
	this.openView(viewFromUrl);
}

ViewManager.prototype.addDropdownItem = function(view) {
	var item = view.listItem = document.createElement('li');
	item.appendChild(document.createTextNode(view.id));
	item.addEventListener('click', _.bind(this.onViewClick, this, view));
	this.viewList.appendChild(item);
}

ViewManager.prototype.onSelectViewClick = function(e) {
	this.viewDropdown.classList.toggle('open');
	this.viewDropdown.style.left = this.selectViewBtn.offsetLeft + 'px';
	this.viewDropdown.style.top = this.selectViewBtn.offsetHeight + 'px';
	e.stopPropagation();
};

ViewManager.prototype.onBodyClick = function() {
	this.viewDropdown.classList.remove('open');
}

ViewManager.prototype.openView = function(view) {
	if ( this.hasChanges && ! confirm('Discard changes?') ) {
		return false;
	}
	this.panelManager.openFromJSONString(view.json);
	this.selectViewBtn.textContent = view.id;
	this.currentView = view;
	this.hasChanges = false;
}

ViewManager.prototype.onViewClick = function(view) {
	this.openView(view);

	if ( window.history && window.history.pushState ) {
		history.pushState({}, document.title, '/view/'+encodeURIComponent(view.id));
	}
}

ViewManager.prototype.onSaveClick = function(e) {
	if ( this.currentView === null ) {
		var name = prompt('Enter a name for the view', '');
		if ( name === null ) return;

		var view = {
			id: name,
			json: '[]'
		};
		this.addDropdownItem(view);
		this.views.push(view);

		this.currentView = view;
		this.selectViewBtn.textContent = view.id;
	}
	this.currentView.json = this.panelManager.openPanelsToJSONString();
	this.ioSocket.emit('views.save', this.currentView.id, this.currentView.json);
	this.hasChanges = false;
}

ViewManager.prototype.onNewClick = function(e) {
	if ( this.hasChanges && ! confirm('Discard changes?') ) {
		return false;
	}
	panelManager.clear();
	this.currentView = null;
	this.selectViewBtn.textContent = 'Open View ...';
	this.hasChanges = false;
}

ViewManager.prototype.onBeforeUnload = function(e) {
	if ( this.hasChanges ) {
		e.returnValue = 'The open view has unsaved changes. Close anyway?';
		return e.returnValue;
	}
}

ViewManager.prototype.onPopState = function() {
	this.loadFromUrl();
}
