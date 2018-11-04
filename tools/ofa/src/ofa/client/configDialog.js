var _ = require('underscore');
var ui = require('./ui').ui;
var debugMan = require('./connection').debugMan;

var configModal = {};
configModal.modal = new ui.Modal('Create Panel');

var typeMap = {
    "number": "float"
};

function niceTypeName(value) {
    var typename = typeof value;
    if (typeMap.hasOwnProperty(typename)) {
        typename = typeMap[typename];
    }
    return typename;
}

function formatKeyType(key) {
    console.log(key);
    if (key.hasOwnProperty('isImage') && key.isImage) return 'image';
    return (_.isArray(key.value) ? 'array(' + key.value.length + ') of ' + niceTypeName(key.value[0]) : niceTypeName(key.value));
}


function createMappingFct(argKeys, body, retTypes) {
    var argCount = argKeys.length;
    var argNames = [];
    for (var i = 0; i < argCount; i++) {
        argNames.push('key' + i);
    }
    try {
        var fun = Function.apply({}, argNames.concat(body));
    } catch (e) {
        alert('Mapping function error: ' + e.message);
        return null;
    }
    var exArgs = _.map(argKeys, getExampleValueForKey);
    try {
        var exRes = fun.apply(this, exArgs);
    } catch (e) {
        alert('Mapping function error: ' + e.message);
        return null;
    }

    var finalReturnType
    var typeCheckPassed = _.any(retTypes, function (retType) {
        if (!_.isArray(exRes) || exRes.length !== retType.length) {
            console.error('Mapping function should return an array of ' + retType.length + ' elements');
            return false;
        }
        var validRetTypes = _.all(_.zip(retType, exRes), checkType);
        if (!validRetTypes) {
            console.error('The return type of the mapping function does not match requirements');
            return false;
        }
        finalReturnType = exRes[0]
        return true;
    });
    if (!typeCheckPassed) {
        alert('No matching type found!');
        return null;
    }
    return [fun, finalReturnType];
}

function getExampleValueForKey(key) {
    if (key.isImage) return getExampleValue('image');
    var isArray = _.isArray(key.value);
    var type = isArray ? niceTypeName(key.value[0]) : niceTypeName(key.value);
    var exValue = getExampleValue(type);
    if (!isArray) return exValue;
    var exArray = [];
    for (var i = 0; i < key.value.length; i++) {
        exArray.push(exValue);
    }
    return exArray;
}

function ImageExample() {
}

function getExampleValue(type) {
    switch (type) {
        case 'bool':
            return true;
        case 'string':
            return 't';
        case 'float':
            return 0.1;
        case 'image':
            return new ImageExample();
        case 'object':
            return {};
        default:
            throw new Error('Unknown type ' + type);
    }
}

function checkType(x) {
    var type = x[0], val = x[1];
    switch (type) {
        case 'bool':
            return _.isBoolean(val);
        case 'string':
            return _.isString(val);
        case '[string]':
            return _.isArray(val) && (val.length == 0 || _.isString(val[0]));
        case 'int':
            return _.isNumber(val) && val == (val | 1);
        case 'float':
            return _.isNumber(val);
        case '[float]':
            return _.isArray(val) && (val.length == 0 || _.isNumber(val[0]));
        case 'vector2':
            return _.isArray(val) && val.length == 2 && _.isNumber(val[0]) && _.isNumber(val[1]);
        case 'vector3':
            return _.isArray(val) && val.length == 3 && _.isNumber(val[0]) && _.isNumber(val[1]) && _.isNumber(val[2]);
        case 'image':
            return val instanceof ImageExample;
        case 'object':
            return _.isObject(val);
        default:
            throw new Error('Unknown type ' + x);
    }
}

function KeySelector() {
    this.init.apply(this, arguments);
}

KeySelector.prototype.init = function (container, preSelect, filter) {
    var keyDD = this.keySelect = document.createElement('select');
    _.chain(debugMan.keyList).sortBy('key').each(function (key) {
        if (filter && filter.indexOf(formatKeyType(key)) < 0) return;
        var elem = document.createElement('option');
        elem.value = key.key;
        elem.appendChild(document.createTextNode(key.key + ' (' + formatKeyType(key) + ')'));
        if (key.key == preSelect) {
            elem.selected = true;
        }
        keyDD.appendChild(elem);
    });
    container.appendChild(keyDD);
};

KeySelector.prototype.remove = function () {
    this.keySelect.parentNode.removeChild(this.keySelect);
};

KeySelector.prototype.onChange = function (handler) {
    this.keySelect.addEventListener('change', handler, false);
};

KeySelector.prototype.value = function () {
    return this.keySelect.value;
};

// ==============

configModal.show = function (ctr, modifyInstance) {
    this.nextCtr = ctr;
    this.modifyInstance = modifyInstance instanceof ctr ? modifyInstance : null;
    var lastConfig = modifyInstance && modifyInstance._lastInitConfig;

    var tabIndex = 1;
    this.keyPressBinding = _.bind(this.onKeyUp, this)

    var body = this.modal.body;
    body.innerHTML = '';

    this.keyFields = [];
    this.keyFieldWrapper = document.createElement('section');
    this.keyFieldWrapper.tabIndex = tabIndex++;
    body.appendChild(this.keyFieldWrapper);

    this.addKeyBtn = ui.createButton(body, 'Add Key', this.addKey, this);
    this.removeKeyBtn = ui.createButton(body, 'Remove Key', this.removeKey, this);

    // === key middleware ===
    body.appendChild(document.createElement('hr'));
    body.appendChild(document.createElement('strong').addText('Map the following key vars ...'));
    this.keyInfoWrapper = document.createElement('section');
    body.appendChild(this.keyInfoWrapper);
    body.appendChild(document.createElement('strong').addText('... to this return type:'));
    body.addText(' ' + _.map(ctr.expectedKeys, function (keys) {
        return '[' + keys.join(', ') + ']';
    }).join(' or '));
    this.mappingBox = document.createElement('textarea');
    this.mappingBox.addEventListener("focus", function () {
        document.documentElement.removeEventListener("keyup", this.keyPressBinding, false)
    }.bind(this));
    this.mappingBox.addEventListener("blur", function () {
        document.documentElement.addEventListener("keyup", this.keyPressBinding, false)
    }.bind(this))
    this.mappingBox.tabIndex = tabIndex++;
    this.mappingBox.rows = 2;
    if (lastConfig) {
        this.mappingBox.addText(lastConfig.mapping);
    } else {
        this.mappingBox.addText('return [key0]');
    }
    body.appendChild(this.mappingBox);

    // === config ===
    body.appendChild(document.createElement('hr'));

    var configOptions = ctr.defaultConfig;
    if (lastConfig) {
        configOptions = _.pick(_.extend({}, ctr.defaultConfig, lastConfig), _.keys(ctr.defaultConfig));
    }
    this.configFields = _.map(configOptions, _.bind(function (defValue, name) {
        var id = _.uniqueId('configField');
        var label = document.createElement('label');
        label.for = id;
        label.appendChild(document.createTextNode(name));
        body.appendChild(label);
        var input = document.createElement('input');
        input.id = id;
        input.name = name;
        if (typeof(defValue) === "boolean") {
            input.type = 'checkbox'
            input.checked = defValue;
        } else
            input.value = defValue;
        input.tabIndex = tabIndex++;
        body.appendChild(input);
        return input;
    }, this));

    // Create putains
    ui.createButton(body, 'OK', this.onOK, this);
    ui.createButton(body, 'Cancel', this.onCancel, this);

    // Show Modal
    if (lastConfig) {
        for (const key of lastConfig.keys) {
            this.addKey(key, this.nextCtr.filter);
        }
    } else {
        this.addKey(undefined, this.nextCtr.filter);
    }
    document.documentElement.addEventListener("keyup", this.keyPressBinding, false);
    this.modal.show();
};

configModal.onKeyUp = function (e) {
    e = e || window.event;
    if (e.keyCode == 13) { // detect enter
        this.onOK();
        return false;
    } else if (e.keyCode == 27) { // detect esc
        this.onCancel()
        return false;
    }
    return true;
}

configModal.hide = function () {
    document.documentElement.removeEventListener("keyup", this.keyPressBinding, false);
    this.modal.hide();
};

configModal.addKey = function (preSelect, filter) {
    var selector = new KeySelector(this.keyFieldWrapper, preSelect, filter);
    selector.onChange(_.bind(this.onKeysChange, this));
    this.keyFields.push(selector);
    this.onKeysChange();
};
configModal.removeKey = function () {
    if (this.keyFields.length <= 1) return;
    var selector = this.keyFields.pop();
    selector.remove();
    this.onKeysChange();
};

configModal.getKeyInfos = function () {
    var keyInfos = [];
    for (var i = 0; i < this.keyFields.length; i++) {
        var key = this.keyFields[i].value();
        keyInfos.push(debugMan.keyList[key]);
    }
    return keyInfos;
};
configModal.onKeysChange = function () {
    this.keyInfoWrapper.innerHTML = '';
    var keyInfos = this.getKeyInfos();
    for (var i = 0; i < keyInfos.length; i++) {
        var keyInfo = keyInfos[i];
        var type = formatKeyType(keyInfo, true);
        var row = document.createElement('div');
        var span = document.createElement('em');
        row.appendChild(document.createElement('strong').addText('key' + i));
        row.addText(': ' + type);
        this.keyInfoWrapper.appendChild(row);
    }
};

configModal.onOK = function () {
    var mapping = this.mappingBox.value;
    var mapped = createMappingFct(this.getKeyInfos(), mapping, this.nextCtr.expectedKeys);
    var mappingFct = mapped[0]
    var returnType = mapped[1]
    if (mappingFct === null) return;
    var keys = _.map(this.keyFields, function (keyField) {
        return keyField.value();
    });

    var config = {
        keys: keys,
        returnType: returnType, //_.pluck(this.getKeyInfos(), 'value'),
        mapping: mapping,
        mappingFct: mappingFct
    };
    _.each(this.configFields, function (field) {
        config[field.name] = (field.type == 'checkbox') ? field.checked : field.value
    });

    if (this.modifyInstance) {
        this.modifyInstance.reInit(config);
    } else {
        new this.nextCtr(config);
    }
    this.hide();
};

configModal.onCancel = function () {
    this.hide();
};

module.exports = configModal;
