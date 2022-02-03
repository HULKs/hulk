import copy
import json
import os
import typing

import PyQt5.QtGui as qtg
import PyQt5.QtWidgets as qtw
import PyQt5.uic

from hulks.json_utils import comment_remover

from mate.debug.colorlog import ColorLog
from mate.net.nao import Nao

logger = ColorLog()


def loadUi(path: str, reference) -> None:
    """
    Loads a ui file with the same name as the given path to the .py file. (e.g. image/main.py -> image/main.ui). The loaded ui file is applied to the given reference.
    :param path: The path to the .py file where the corresponding eponymous .ui file is
    :param reference: The reference to the QtObject to which it is to be applied
    """
    PyQt5.uic.loadUi(path[0:-3] + ".ui", reference)


def ideal_text_color(bg_color: qtg.QColor):
    """
    function to get the ideal text color (black/white) for a background
    @param bg_color
    @return String of the ideal text color in hex
    """
    threshold = 150
    background_delta = (bg_color.red() * 0.299) + \
                       (bg_color.green() * 0.587) + \
                       (bg_color.blue() * 0.114)
    if 255 - background_delta < threshold:
        return "#000000"
    else:
        return "#ffffff"


def reset_textField_color(tf: qtw.QLineEdit, color: str):
    """
    function for conveniently using a text-field for colors.
    Sets the text, bg_color and text-color to accomodate a given color.
    @param tf
    @param color
    """
    tf.setText(color)
    tf.setStyleSheet(
        "background-color: {}; color: {}; border: 1px solid".format(
            color, ideal_text_color(qtg.QColor(color))))
    tf.setAutoFillBackground(True)


def pick_color(tf: qtw.QLineEdit, old_color: str):
    """
    function to open a color-picker for a color text-field.
    @param tf
    @param old_color
    """
    color = qtw.QColorDialog.getColor(qtg.QColor(old_color))
    if color.isValid():
        reset_textField_color(tf, color.name())


def init_Color_UI(btn: qtw.QPushButton, tf: qtw.QLineEdit):
    """
    function to initialize a "Pick" button together with a text-field,
    to get a working ui element for a color.
    @param btn
    @param tf
    """
    btn.pressed.connect(lambda: pick_color(tf, tf.text()))
    tf.returnPressed.connect(lambda: reset_textField_color(tf, tf.text()))


def init_cbx(cbx: qtw.QComboBox, cbx_key, debug_data):
    """
    function to initialize a combo-box for debug-keys
    @param cbx
    @param key
    @param debug_data
    """
    cbx.setMinimumContentsLength(1)
    cbx.clear()
    if cbx_key not in debug_data:
        cbx.addItem(cbx_key)
    for key, data in debug_data.items():
        if not data.isImage:
            cbx.addItem(key)
    cbx.setCurrentText(cbx_key)


def recursive_merge_dicts(base, new):
    result = base.copy()
    for key, value in new.items():
        if isinstance(value, dict):
            # get node or create one
            node = result.setdefault(key, {})
            result[key] = recursive_merge_dicts(node, value)
        else:
            result[key] = value
    return result


def load_model(model_file: str, saved_model: typing.Dict = None):
    with open(model_file) as fp:
        json_model = json.load(fp)

    if saved_model is None:
        return json_model
    else:
        merged_model = recursive_merge_dicts(json_model, saved_model)
        if "config" in json_model and saved_model.get("config", None) is None:
            merged_model["config"] = json_model["config"]
        return merged_model


def parse_json_file(path: str) -> dict:
    """Parse a json file. Returns empty dict when fails"""
    data = {}
    try:
        with open(path, "r") as f:
            data = json.loads(comment_remover(f.read()))
        logger.debug(__name__ + ": File: " + path + " found.")
    except json.JSONDecodeError:
        logger.debug(__name__ + ": File: " + path +
                     " doesn't seem to be a valid json file.")
    except FileNotFoundError:
        logger.debug(__name__ + ": File: " + path + " not found.")
    return data


def merge_json_dicts(orig: dict, diff: dict) -> dict:
    """Overwrite values in orig with values from diff"""
    for key in diff.keys():
        orig[key] = diff[key]
    return orig


def get_file_location(panel: qtw.QFileDialog, caption: str, suggestion: str) -> str:
    location, _ = qtw.QFileDialog. \
        getSaveFileName(panel,
                        caption=caption,
                        directory=suggestion,
                        options=qtw.QFileDialog.Options())
    return location


def save_dict_to_file(location, data: dict) -> bool:
    try:
        with open(location, 'w') as f:
            json.dump(data, f, indent=4, sort_keys=True)
            f.write("\n")
        return True
    except Exception as e:
        logger.error(
            __name__ + ": Exception while saving config to file: " + str(e))
        return False


class ConfigDiffInfoContainer:

    def __init__(self, config_dir: str, mount: str, nao: Nao, mode: int):
        path_join = os.path.join
        self.mount = mount
        self.mode = mode
        # Get base directory as absolute path
        self.base_path = os.path.abspath(config_dir)
        # Get info about Nao parts and location
        self.nao_head = nao.nao_head if nao.nao_head != None else "99"
        self.nao_body = nao.nao_body if nao.nao_body != None else "99"
        self.location = nao.location if nao.location != None else "fallback"
        if "99" in [self.nao_head, self.nao_body] or self.location == "fallback":
            logger.warning("Export Diff: No head/body/location information "
                           "received. Using fallback.")
        self.location_path = path_join("location", self.location)
        self.location_default_path = "location/default"
        # Get relevant part of mount path
        self.mount_split_path = self.mount.split("/")
        self.relevant_path = self.mount_split_path[self.mount_split_path.index(
            "location"):]
        # Get filename
        self.filename = self.relevant_path[-1]
        # Get head/body path part
        self.head_or_body = self.relevant_path[2]
        self.is_head = True if self.head_or_body == "head" else False
        self.head_body_default_path = path_join(self.head_or_body, "default")
        self.head_body_path = path_join(self.head_or_body,
                                        self.nao_head if self.is_head else self.nao_body)
        # Paths to json files
        self.path_root_json = path_join(
            self.base_path, self.location_default_path, self.filename
        )
        self.paths = [
            path_join(
                self.base_path, self.location_default_path, self.head_body_default_path,
                self.filename),
            path_join(
                self.base_path, self.location_default_path, self.head_body_path, self.filename)
        ]
        if self.location != "default":
            self.paths.append(path_join(
                self.base_path, self.location_path, self.head_body_default_path, self.filename
            ))
            self.paths.append(path_join(
                self.base_path, self.location_path, self.head_body_path, self.filename
            ))


def get_default_config(info: ConfigDiffInfoContainer) -> dict:
    """Get default values for ConfigMount"""
    # If relevant path is too short, it is not something we want to diff
    if len(info.relevant_path) <= 3:
        logger.info(__name__ + ": You can't diff a tuhhSDK mount.")
        return {}

    # List to store all JSON Objects that exist already
    # The last one will be discarded and is to be replaced
    data = {}
    data_list = list()

    # # Go from most generic to most specific configuration # #
    # Append most default root config
    data_list.append(parse_json_file(info.path_root_json))

    # If generating Location: specific Nao: default, we want to ignore
    # Location: default Nao: specific
    path_list = copy.deepcopy(info.paths)
    if info.mode == 2:
        path_list.pop(2)

    # Read all relevant files and append to list
    for i in range(0, info.mode):
        data_list.append(parse_json_file(path_list[i]))

    # Apply all the diffs
    for diff in data_list:
        data = merge_json_dicts(data, diff)

    if data == {}:
        logger.error(__name__ + ": No data found for mount " + info.mount)

    # Now data should be the default config for location X to diff against
    return data
