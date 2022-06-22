SUMMARY = "Nao SPL Robocup -- HULKs setup"
HOMEPAGE = "https://hulks.de"
LICENSE = "CLOSED"

SRC_URI = " \
            file://hulk.service \
            file://hulk-gdbserver.service \
            file://launchHULK \
            file://hulk \
          "

SYSTEMD_PACKAGES = "${PN}"
SYSTEMD_SERVICE:${PN} = "hulk.service hulk-gdbserver.service"

inherit systemd

do_install() {
  install -d ${D}${bindir}
  install -m 755 ${WORKDIR}/launchHULK ${D}${bindir}
  install -m 755 ${WORKDIR}/hulk ${D}${bindir}

  install -d ${D}${systemd_unitdir}/system/
  install -m 0644 ${WORKDIR}/hulk.service ${D}${systemd_unitdir}/system/
  install -m 0644 ${WORKDIR}/hulk-gdbserver.service ${D}${systemd_unitdir}/system/
}

FILES:${PN} = "\
                ${bindir}/camera-reset \
                ${bindir}/launchHULK \
                ${bindir}/hulk \
                ${systemd_unitdir}/system/hulk.service \
                ${systemd_unitdir}/system/hulk-gdbserver.service \
              "
