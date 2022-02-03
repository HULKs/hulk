SUMMARY = "Add systemd services to initially set ip address config"
LICENSE = "CLOSED"

SRC_URI = "\
           file://id_map.json \
           file://configure_network \
           file://network-config.service \
           "

do_install() {
    install -d ${D}${sysconfdir}/
    install -m 0644 ${WORKDIR}/id_map.json ${D}${sysconfdir}/
    install -d ${D}${sbindir}/
    install -m 0755 ${WORKDIR}/configure_network ${D}${sbindir}/
    install -d ${D}${systemd_unitdir}/system/
    install -m 0644 ${WORKDIR}/network-config.service ${D}${systemd_unitdir}/system/
}

FILES:${PN} = "\
               ${sysconfdir}/id_map.json \
               ${sbindir}/configure_network \
               ${systemd_unitdir}/system/network-config.service \
              "

# install services by default
#NATIVE_SYSTEMD_SUPPORT = "1"
SYSTEMD_PACKAGES = "${PN}"
SYSTEMD_SERVICE:${PN} = "network-config.service"

inherit systemd
