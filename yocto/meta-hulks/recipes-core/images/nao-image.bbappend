FILESEXTRAPATHS:prepend := "${THISDIR}/${PN}:"

CORE_IMAGE_EXTRA_INSTALL += "\
                             alsa-lib \
                             alsa-state \
                             boost \
                             bzip2 \
                             compilednn \
                             fftw \
                             hula \
                             hulk \
                             jq \
                             libjpeg-turbo \
                             libogg \
                             libopus \
                             libpng \
                             libsndfile1 \
                             libxml2-utils \
                             nano \
                             network-config \
                             opusfile \
                             zlib \
                            "

TOOLCHAIN_TARGET_TASK:append = " libeigen-dev"
TOOLCHAIN_HOST_TASK:append = " packagegroup-rust-cross-canadian-${MACHINE}"
