import PyQt5.QtWidgets as qtw
import PyQt5.QtGui as qtg


def ideal_text_color(bg_color: qtg.QColor):
    """
    function to get the ideal text color (black/white) for a background
    @param bg_color
    @return String of the ideal text color in hex
    """
    threshold = 150
    background_delta = (bg_color.red() * 0.299) + \
        (bg_color.green() * 0.587) + (bg_color.blue() * 0.114)
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
