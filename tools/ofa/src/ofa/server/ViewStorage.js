var fs = require('fs');
var path = require('path');
var utils = require('util');
var EventEmitter = require('events').EventEmitter;
var asyncLib = require('async');

function ViewStorage() {
    if (!(this instanceof ViewStorage)) {
        throw new Exception('ViewStorage is a constructor and needs to be called with keyword "new"');
    }
    EventEmitter.call(this);
    this.init.apply(this, arguments);
}

utils.inherits(ViewStorage, EventEmitter);

ViewStorage.prototype.init = function (io) {
    this.io = io;
    this.views = {};
    this.loadViews(function () {
        this.emit('ready');
    }.bind(this));
    io.on('connection', this.onConnection.bind(this));
};

ViewStorage.prototype.loadViews = function (done) {
    fs.readdir(path.join('.', 'views'), function (err, files) {
        if (err) return done(err);
        asyncLib.each(files, function (filename, done) {
            if (!/^.+\.json$/.test(filename)) {
                return done();
            }
            fs.readFile(path.join('.', 'views', filename), {encoding: 'utf8'}, function (err, data) {
                if (err) {
                    return done(err);
                }
                var id = filename.match(/^(.+)\.json$/)[1];
                this.views[id] = data;
                done();
            }.bind(this));
        }.bind(this), done);
    }.bind(this));
};

ViewStorage.prototype.onConnection = function (socket) {
    socket.emit('views.update', this.views);
    socket.on('views.save', this.onViewSave.bind(this));
};

ViewStorage.prototype.onViewSave = function (id, json) {
    this.views[id] = json;
    fs.writeFile(path.join('.', 'views', id + '.json'), json, {encoding: 'utf8'});
}

module.exports = ViewStorage;
