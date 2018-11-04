# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'src/mate/ui/views/map/config_view.ui'
#
# Created by: PyQt5 UI code generator 5.9.2
#
# WARNING! All changes made in this file will be lost!

from PyQt5 import QtCore, QtGui, QtWidgets

class Ui_Config(object):
    def setupUi(self, Config):
        Config.setObjectName("Config")
        Config.resize(455, 595)
        self.verticalLayout = QtWidgets.QVBoxLayout(Config)
        self.verticalLayout.setObjectName("verticalLayout")
        self.groupBox = QtWidgets.QGroupBox(Config)
        self.groupBox.setObjectName("groupBox")
        self.verticalLayout_2 = QtWidgets.QVBoxLayout(self.groupBox)
        self.verticalLayout_2.setContentsMargins(0, 0, 0, 0)
        self.verticalLayout_2.setObjectName("verticalLayout_2")
        self.formLayout = QtWidgets.QFormLayout()
        self.formLayout.setContentsMargins(4, 4, 4, 4)
        self.formLayout.setObjectName("formLayout")
        self.viewportLabel = QtWidgets.QLabel(self.groupBox)
        self.viewportLabel.setObjectName("viewportLabel")
        self.formLayout.setWidget(0, QtWidgets.QFormLayout.LabelRole, self.viewportLabel)
        self.viewportWidget = QtWidgets.QWidget(self.groupBox)
        self.viewportWidget.setObjectName("viewportWidget")
        self.horizontalLayout_2 = QtWidgets.QHBoxLayout(self.viewportWidget)
        self.horizontalLayout_2.setContentsMargins(0, 0, 0, 0)
        self.horizontalLayout_2.setObjectName("horizontalLayout_2")
        self.label_width = QtWidgets.QLabel(self.viewportWidget)
        self.label_width.setAlignment(QtCore.Qt.AlignRight|QtCore.Qt.AlignTrailing|QtCore.Qt.AlignVCenter)
        self.label_width.setObjectName("label_width")
        self.horizontalLayout_2.addWidget(self.label_width)
        self.spin_viewport_width = QtWidgets.QDoubleSpinBox(self.viewportWidget)
        self.spin_viewport_width.setObjectName("spin_viewport_width")
        self.horizontalLayout_2.addWidget(self.spin_viewport_width)
        self.label_height = QtWidgets.QLabel(self.viewportWidget)
        self.label_height.setAlignment(QtCore.Qt.AlignRight|QtCore.Qt.AlignTrailing|QtCore.Qt.AlignVCenter)
        self.label_height.setObjectName("label_height")
        self.horizontalLayout_2.addWidget(self.label_height)
        self.spin_viewport_height = QtWidgets.QDoubleSpinBox(self.viewportWidget)
        self.spin_viewport_height.setObjectName("spin_viewport_height")
        self.horizontalLayout_2.addWidget(self.spin_viewport_height)
        self.formLayout.setWidget(0, QtWidgets.QFormLayout.FieldRole, self.viewportWidget)
        self.fpsLabel = QtWidgets.QLabel(self.groupBox)
        self.fpsLabel.setObjectName("fpsLabel")
        self.formLayout.setWidget(1, QtWidgets.QFormLayout.LabelRole, self.fpsLabel)
        self.spin_fps = QtWidgets.QSpinBox(self.groupBox)
        self.spin_fps.setObjectName("spin_fps")
        self.formLayout.setWidget(1, QtWidgets.QFormLayout.FieldRole, self.spin_fps)
        self.verticalLayout_2.addLayout(self.formLayout)
        self.verticalLayout.addWidget(self.groupBox)
        self.horizontalLayout = QtWidgets.QHBoxLayout()
        self.horizontalLayout.setObjectName("horizontalLayout")
        self.btnAccept = QtWidgets.QPushButton(Config)
        self.btnAccept.setObjectName("btnAccept")
        self.horizontalLayout.addWidget(self.btnAccept)
        self.btnDiscard = QtWidgets.QPushButton(Config)
        self.btnDiscard.setObjectName("btnDiscard")
        self.horizontalLayout.addWidget(self.btnDiscard)
        self.verticalLayout.addLayout(self.horizontalLayout)
        spacerItem = QtWidgets.QSpacerItem(20, 40, QtWidgets.QSizePolicy.Minimum, QtWidgets.QSizePolicy.Expanding)
        self.verticalLayout.addItem(spacerItem)

        self.retranslateUi(Config)
        QtCore.QMetaObject.connectSlotsByName(Config)
        Config.setTabOrder(self.btnAccept, self.btnDiscard)

    def retranslateUi(self, Config):
        _translate = QtCore.QCoreApplication.translate
        Config.setWindowTitle(_translate("Config", "Form"))
        self.groupBox.setTitle(_translate("Config", "General:"))
        self.viewportLabel.setText(_translate("Config", "viewport"))
        self.label_width.setText(_translate("Config", "Width:"))
        self.label_height.setText(_translate("Config", "Height:"))
        self.fpsLabel.setText(_translate("Config", "fps"))
        self.btnAccept.setText(_translate("Config", "Accept"))
        self.btnDiscard.setText(_translate("Config", "Discard"))

