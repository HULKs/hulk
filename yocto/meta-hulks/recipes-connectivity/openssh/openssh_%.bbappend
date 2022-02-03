FILESEXTRAPATHS:prepend := "${THISDIR}/${PN}:"
SRC_URI:append = "\
           file://authorized_keys \
           "

do_install:append() {
    install -d ${D}${sysconfdir}/ssh
    install -m 0644 ${WORKDIR}/authorized_keys ${D}${sysconfdir}/ssh/
    sed -i -e 's:AuthorizedKeysFile.*:AuthorizedKeysFile /etc/ssh/authorized_keys .ssh/authorized_keys:g' ${D}${sysconfdir}/ssh/sshd_config
}

FILES:${PN} = "\
               ${sysconfdir}/ssh/authorized_keys \
              "
