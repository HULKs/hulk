var CSVExport = function(){
	this.init.apply(this, arguments);
};
util.inherits(CSVExport, DebugDisplay);

CSVExport.defaultConfig = {
	title : ''
};

CSVExport.expectedKeys = [['float', 'float', 'float']];
CSVExport.DialogName = 'CSVExport';

CSVExport.prototype.createButton = function (buttonContainer, text, callback) {
	var clearBtn = document.createElement('button');
	clearBtn.appendChild(document.createTextNode(text));
	clearBtn.addEventListener('click', callback, false);
	buttonContainer.appendChild(clearBtn);
};

CSVExport.prototype.onRecordClick = function () {
	this.hot = !this.hot;
};

CSVExport.prototype.init = function(config){
	DebugDisplay.prototype.init.call(this, config);

	this.onUpdate = _.bind(this.onUpdate, this);

	this.config = _.defaults(config||{}, CSVExport.defaultConfig);
	this.buffer = new Array();
	this.hot = false;

	this.wrapper.classList.add('blob');
	this.textBox = document.createElement('textarea');
	this.wrapper.appendChild(this.textBox);

	var buttonContainer = document.createElement('div');
	buttonContainer.className="wrapper container";

	this.createButton(buttonContainer, 'Dump', _.bind(this.onButtonClick, this));
	this.createButton(buttonContainer, 'clear', _.bind(this.onClearClick, this));
	this.createButton(buttonContainer, 'record', _.bind(this.onRecordClick, this));

	this.wrapper.appendChild(buttonContainer);

	this.subscribe(config.keys, config.mappingFct);
};

CSVExport.prototype.onButtonClick = function () {
	this.dump();
};


CSVExport.prototype.onUpdate = function(f1, f2, f3){
    if(this.hot){
        this.buffer.push([f1, f2, f3]);
    }
};

CSVExport.prototype.dump = function() {
    var t = 0;
    var times = '';
    var data1 = '';
    var data2 = '';
    var data3 = '';
    _.each(this.buffer, function(val){
        t += 0.01;
        times += t.toString() + ' ';
        data1 += val[0].toString() + ' ';
        data2 += val[1].toString() + ' ';
        data3 += val[2].toString() + ' ';
    });

    times = 'times = [' + times + '];\n';
    data1 = 'data1 = [' + data1 + '];\n';
    data2 = 'data2 = [' + data2 + '];\n';
    data3 = 'data3 = [' + data3 + '];';

    this.textBox.innerHTML = times + data1 + data2 + data3;
};

CSVExport.prototype.onClearClick = function(){
	this.textBox.innerHTML = '';
	this.buffer = new Array();
};
