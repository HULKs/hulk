# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'tools/mate/src/mate/ui/views/view/view_view.ui'
#
# Created by: PyQt5 UI code generator 5.9
#
# WARNING! All changes made in this file will be lost!

from PyQt5 import QtCore, QtGui, QtWidgets

class Ui_DebugView(object):
    def setupUi(self, DebugView):
        DebugView.setObjectName("DebugView")
        DebugView.resize(400, 300)
        self.dockWidgetContents = QtWidgets.QWidget()
        self.dockWidgetContents.setObjectName("dockWidgetContents")
        self.verticalLayout = QtWidgets.QVBoxLayout(self.dockWidgetContents)
        self.verticalLayout.setContentsMargins(0, 0, 0, 0)
        self.verticalLayout.setObjectName("verticalLayout")
        self.horizontalLayout = QtWidgets.QHBoxLayout()
        self.horizontalLayout.setObjectName("horizontalLayout")
        self.cbxMount = QtWidgets.QComboBox(self.dockWidgetContents)
        self.cbxMount.setEditable(True)
        self.cbxMount.setObjectName("cbxMount")
        self.horizontalLayout.addWidget(self.cbxMount)
        self.btnSnap = QtWidgets.QPushButton(self.dockWidgetContents)
        self.btnSnap.setMinimumSize(QtCore.QSize(30, 0))
        self.btnSnap.setMaximumSize(QtCore.QSize(30, 16777215))
        self.btnSnap.setObjectName("btnSnap")
        self.horizontalLayout.addWidget(self.btnSnap)
        self.label_2 = QtWidgets.QLabel(self.dockWidgetContents)
        self.label_2.setMinimumSize(QtCore.QSize(25, 0))
        self.label_2.setMaximumSize(QtCore.QSize(25, 16777215))
        self.label_2.setObjectName("label_2")
        self.horizontalLayout.addWidget(self.label_2)
        self.spnFramerate = QtWidgets.QSpinBox(self.dockWidgetContents)
        self.spnFramerate.setMinimumSize(QtCore.QSize(50, 0))
        self.spnFramerate.setMaximumSize(QtCore.QSize(50, 16777215))
        self.spnFramerate.setProperty("value", 30)
        self.spnFramerate.setObjectName("spnFramerate")
        self.horizontalLayout.addWidget(self.spnFramerate)
        self.verticalLayout.addLayout(self.horizontalLayout)
        self.scrollArea = QtWidgets.QScrollArea(self.dockWidgetContents)
        self.scrollArea.setWidgetResizable(True)
        self.scrollArea.setObjectName("scrollArea")
        self.scrollAreaWidgetContents = QtWidgets.QWidget()
        self.scrollAreaWidgetContents.setGeometry(QtCore.QRect(0, 0, 386, 241))
        self.scrollAreaWidgetContents.setObjectName("scrollAreaWidgetContents")
        self.verticalLayout_2 = QtWidgets.QVBoxLayout(self.scrollAreaWidgetContents)
        self.verticalLayout_2.setContentsMargins(0, 0, 0, 0)
        self.verticalLayout_2.setObjectName("verticalLayout_2")
        self.label = QtWidgets.QLabel(self.scrollAreaWidgetContents)
        sizePolicy = QtWidgets.QSizePolicy(QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Expanding)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.label.sizePolicy().hasHeightForWidth())
        self.label.setSizePolicy(sizePolicy)
        self.label.setText("")
        self.label.setTextInteractionFlags(QtCore.Qt.TextSelectableByKeyboard|QtCore.Qt.TextSelectableByMouse)
        self.label.setObjectName("label")
        self.verticalLayout_2.addWidget(self.label)
        self.scrollArea.setWidget(self.scrollAreaWidgetContents)
        self.verticalLayout.addWidget(self.scrollArea)
        DebugView.setWidget(self.dockWidgetContents)

        self.retranslateUi(DebugView)
        QtCore.QMetaObject.connectSlotsByName(DebugView)

    def retranslateUi(self, DebugView):
        _translate = QtCore.QCoreApplication.translate
        DebugView.setWindowTitle(_translate("DebugView", "DockWidget"))
        self.btnSnap.setText(_translate("DebugView", "Snap"))
        self.label_2.setText(_translate("DebugView", "fps:"))

