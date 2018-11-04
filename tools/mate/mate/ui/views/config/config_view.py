# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'src/mate/ui/views/config/config_view.ui'
#
# Created by: PyQt5 UI code generator 5.10.1
#
# WARNING! All changes made in this file will be lost!

from PyQt5 import QtCore, QtGui, QtWidgets

class Ui_DockWidget(object):
    def setupUi(self, DockWidget):
        DockWidget.setObjectName("DockWidget")
        DockWidget.resize(400, 300)
        self.dockWidgetContents = QtWidgets.QWidget()
        self.dockWidgetContents.setObjectName("dockWidgetContents")
        self.verticalLayout = QtWidgets.QVBoxLayout(self.dockWidgetContents)
        self.verticalLayout.setObjectName("verticalLayout")
        self.cbxMount = QtWidgets.QComboBox(self.dockWidgetContents)
        self.cbxMount.setEditable(True)
        self.cbxMount.setObjectName("cbxMount")
        self.verticalLayout.addWidget(self.cbxMount)
        self.tblConfig = QtWidgets.QTableWidget(self.dockWidgetContents)
        self.tblConfig.setObjectName("tblConfig")
        self.tblConfig.setColumnCount(0)
        self.tblConfig.setRowCount(0)
        self.verticalLayout.addWidget(self.tblConfig)
        self.horizontalLayout = QtWidgets.QHBoxLayout()
        self.horizontalLayout.setObjectName("horizontalLayout")
        self.btnSet = QtWidgets.QPushButton(self.dockWidgetContents)
        self.btnSet.setObjectName("btnSet")
        self.horizontalLayout.addWidget(self.btnSet)
        self.btnSave = QtWidgets.QPushButton(self.dockWidgetContents)
        self.btnSave.setObjectName("btnSave")
        self.horizontalLayout.addWidget(self.btnSave)
        self.btnExport = QtWidgets.QPushButton(self.dockWidgetContents)
        self.btnExport.setObjectName("btnExport")
        self.horizontalLayout.addWidget(self.btnExport)
        self.btnExportDiff = QtWidgets.QPushButton(self.dockWidgetContents)
        self.btnExportDiff.setObjectName("btnExportDiff")
        self.horizontalLayout.addWidget(self.btnExportDiff)
        self.verticalLayout.addLayout(self.horizontalLayout)
        DockWidget.setWidget(self.dockWidgetContents)

        self.retranslateUi(DockWidget)
        QtCore.QMetaObject.connectSlotsByName(DockWidget)

    def retranslateUi(self, DockWidget):
        _translate = QtCore.QCoreApplication.translate
        DockWidget.setWindowTitle(_translate("DockWidget", "Config"))
        self.btnSet.setText(_translate("DockWidget", "Set"))
        self.btnSave.setText(_translate("DockWidget", "Save"))
        self.btnExport.setText(_translate("DockWidget", "Export"))
        self.btnExportDiff.setText(_translate("DockWidget", "Export Diff"))

