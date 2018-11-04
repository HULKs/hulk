# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'layer_view.ui'
#
# Created by: PyQt5 UI code generator 5.9.2
#
# WARNING! All changes made in this file will be lost!

from PyQt5 import QtCore, QtGui, QtWidgets

class Ui_Layer(object):
    def setupUi(self, Layer):
        Layer.setObjectName("Layer")
        Layer.resize(400, 300)
        self.verticalLayout_2 = QtWidgets.QVBoxLayout(Layer)
        self.verticalLayout_2.setObjectName("verticalLayout_2")
        self.splitter = QtWidgets.QSplitter(Layer)
        self.splitter.setOrientation(QtCore.Qt.Horizontal)
        self.splitter.setObjectName("splitter")
        self.layoutWidget = QtWidgets.QWidget(self.splitter)
        self.layoutWidget.setObjectName("layoutWidget")
        self.verticalLayout = QtWidgets.QVBoxLayout(self.layoutWidget)
        self.verticalLayout.setContentsMargins(0, 0, 0, 0)
        self.verticalLayout.setObjectName("verticalLayout")
        self.listWidget = QtWidgets.QListWidget(self.layoutWidget)
        self.listWidget.setObjectName("listWidget")
        self.verticalLayout.addWidget(self.listWidget)
        self.horizontalLayout = QtWidgets.QHBoxLayout()
        self.horizontalLayout.setObjectName("horizontalLayout")
        self.btnAddLayer = QtWidgets.QToolButton(self.layoutWidget)
        self.btnAddLayer.setPopupMode(QtWidgets.QToolButton.InstantPopup)
        self.btnAddLayer.setToolButtonStyle(QtCore.Qt.ToolButtonTextOnly)
        self.btnAddLayer.setObjectName("btnAddLayer")
        self.horizontalLayout.addWidget(self.btnAddLayer)
        self.btnDeleteLayer = QtWidgets.QPushButton(self.layoutWidget)
        self.btnDeleteLayer.setObjectName("btnDeleteLayer")
        self.horizontalLayout.addWidget(self.btnDeleteLayer)
        self.btnMoveUp = QtWidgets.QPushButton(self.layoutWidget)
        self.btnMoveUp.setObjectName("btnMoveUp")
        self.horizontalLayout.addWidget(self.btnMoveUp)
        self.btnMoveDown = QtWidgets.QPushButton(self.layoutWidget)
        self.btnMoveDown.setObjectName("btnMoveDown")
        self.horizontalLayout.addWidget(self.btnMoveDown)
        self.verticalLayout.addLayout(self.horizontalLayout)
        self.widget = QtWidgets.QWidget(self.splitter)
        self.widget.setObjectName("widget")
        self.verticalLayout_2.addWidget(self.splitter)

        self.retranslateUi(Layer)
        QtCore.QMetaObject.connectSlotsByName(Layer)

    def retranslateUi(self, Layer):
        _translate = QtCore.QCoreApplication.translate
        Layer.setWindowTitle(_translate("Layer", "Form"))
        self.btnAddLayer.setText(_translate("Layer", "Add"))
        self.btnDeleteLayer.setText(_translate("Layer", "Delete"))
        self.btnMoveUp.setText(_translate("Layer", "Up"))
        self.btnMoveDown.setText(_translate("Layer", "Down"))

