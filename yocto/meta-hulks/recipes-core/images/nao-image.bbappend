FILESEXTRAPATHS:prepend := "${THISDIR}/${PN}:"

CORE_IMAGE_EXTRA_INSTALL += "\
                             alsa-lib \
                             alsa-state \
                             compilednn \
                             hula \
                             hulk \
                             jq \
                             libxml2-utils \
                             nano \
                             network-config \
                            "
