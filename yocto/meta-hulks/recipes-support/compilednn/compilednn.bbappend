FILESEXTRAPATHS:prepend := "${THISDIR}/${PN}:"

SRC_URI = "git://github.com/HULKs/CompiledNN.git;branch=thinterface;protocol=https"

SRCREV = "3eb104a9f1283bf238620b6a467dfe63a2d36376"

EXTRA_OECMAKE = "-DBUILD_SHARED_LIBS=ON -DWITH_ONNX=OFF"
