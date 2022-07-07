inherit cargo

SRC_URI += " \
    git://git@github.com/HULKs/nao.git;protocol=ssh;branch=main; \
    file://hula.service \
"
SRCREV = "b0259beb30411f2c36de89fadd25e54a53b55b17"
S = "${WORKDIR}/git/tools/hula"
CARGO_SRC_DIR = ""

SRC_URI += " \
    crate://crates.io/ahash/0.7.6 \
    crate://crates.io/aho-corasick/0.7.18 \
    crate://crates.io/anyhow/1.0.58 \
    crate://crates.io/atty/0.2.14 \
    crate://crates.io/autocfg/1.0.1 \
    crate://crates.io/bitflags/1.3.2 \
    crate://crates.io/build-env/0.3.1 \
    crate://crates.io/byteorder/1.4.3 \
    crate://crates.io/cfg-if/1.0.0 \
    crate://crates.io/chrono/0.4.19 \
    crate://crates.io/clap/3.2.6 \
    crate://crates.io/clap_lex/0.2.3 \
    crate://crates.io/cstr-argument/0.1.2 \
    crate://crates.io/ctrlc/3.2.2 \
    crate://crates.io/dbus/0.9.5 \
    crate://crates.io/dlv-list/0.3.0 \
    crate://crates.io/epoll/4.3.1 \
    crate://crates.io/fern/0.6.1 \
    crate://crates.io/foreign-types/0.5.0 \
    crate://crates.io/foreign-types-macros/0.2.2 \
    crate://crates.io/foreign-types-shared/0.3.1 \
    crate://crates.io/getrandom/0.2.3 \
    crate://crates.io/glob/0.3.0 \
    crate://crates.io/hashbrown/0.12.1 \
    crate://crates.io/hermit-abi/0.1.19 \
    crate://crates.io/hostname/0.3.1 \
    crate://crates.io/indexmap/1.9.1 \
    crate://crates.io/ipnetwork/0.19.0 \
    crate://crates.io/itoa/1.0.2 \
    crate://crates.io/libc/0.2.126 \
    crate://crates.io/libdbus-sys/0.2.2 \
    crate://crates.io/libsystemd-sys/0.9.3 \
    crate://crates.io/log/0.4.17 \
    crate://crates.io/match_cfg/0.1.0 \
    crate://crates.io/memchr/2.4.1 \
    crate://crates.io/memoffset/0.6.4 \
    crate://crates.io/nix/0.24.1 \
    crate://crates.io/no-std-net/0.6.0 \
    crate://crates.io/num-integer/0.1.44 \
    crate://crates.io/num-traits/0.2.14 \
    crate://crates.io/once_cell/1.12.0 \
    crate://crates.io/ordered-multimap/0.4.3 \
    crate://crates.io/os_str_bytes/6.1.0 \
    crate://crates.io/paste/1.0.7 \
    crate://crates.io/pkg-config/0.3.22 \
    crate://crates.io/pnet/0.31.0 \
    crate://crates.io/pnet_base/0.31.0 \
    crate://crates.io/pnet_datalink/0.31.0 \
    crate://crates.io/pnet_macros/0.31.0 \
    crate://crates.io/pnet_macros_support/0.31.0 \
    crate://crates.io/pnet_packet/0.31.0 \
    crate://crates.io/pnet_sys/0.31.0 \
    crate://crates.io/pnet_transport/0.31.0 \
    crate://crates.io/proc-macro2/1.0.40 \
    crate://crates.io/quote/1.0.20 \
    crate://crates.io/regex/1.5.6 \
    crate://crates.io/regex-syntax/0.6.26 \
    crate://crates.io/rmp/0.8.11 \
    crate://crates.io/rmp-serde/1.1.0 \
    crate://crates.io/rust-ini/0.18.0 \
    crate://crates.io/ryu/1.0.5 \
    crate://crates.io/serde/1.0.137 \
    crate://crates.io/serde_derive/1.0.137 \
    crate://crates.io/serde_json/1.0.81 \
    crate://crates.io/strsim/0.10.0 \
    crate://crates.io/syn/1.0.98 \
    crate://crates.io/systemd/0.10.0 \
    crate://crates.io/termcolor/1.1.3 \
    crate://crates.io/textwrap/0.15.0 \
    crate://crates.io/time/0.1.44 \
    crate://crates.io/unicode-ident/1.0.1 \
    crate://crates.io/utf8-cstr/0.1.6 \
    crate://crates.io/version_check/0.9.4 \
    crate://crates.io/wasi/0.10.0+wasi-snapshot-preview1 \
    crate://crates.io/winapi/0.3.9 \
    crate://crates.io/winapi-i686-pc-windows-gnu/0.4.0 \
    crate://crates.io/winapi-util/0.1.5 \
    crate://crates.io/winapi-x86_64-pc-windows-gnu/0.4.0 \
"

LIC_FILES_CHKSUM = " \
    "

HOMEPAGE = "github.com/HULKs/nao"
LICENSE = "GPL-3.0-only"

inherit pkgconfig

DEPENDS += " \
            dbus \
            systemd \
           "
RDEPENDS:${PN} += " \
                   dbus \
                   systemd \
                  "

SYSTEMD_PACKAGES = "${PN}"
SYSTEMD_SERVICE:${PN} = "hula.service"

do_install:append() {
  install -d ${D}${systemd_unitdir}/system/
  install -m 0644 ${WORKDIR}/hula.service ${D}${systemd_unitdir}/system/
}

FILES:${PN} += "${systemd_unitdir}/system/hula.service"

inherit systemd
