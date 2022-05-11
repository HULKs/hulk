inherit cargo

SRC_URI += " \
    git://git@github.com/HULKs/nao.git;protocol=ssh;branch=main; \
    file://hula.service \
"
SRCREV = "86b049724531a29f31c11c196cc8df3541583b79"
S = "${WORKDIR}/git/tools/hula"
CARGO_SRC_DIR = ""

SRC_URI += " \
    crate://crates.io/ahash/0.4.7 \
    crate://crates.io/aho-corasick/0.7.18 \
    crate://crates.io/ansi_term/0.11.0 \
    crate://crates.io/anyhow/1.0.44 \
    crate://crates.io/atty/0.2.14 \
    crate://crates.io/autocfg/1.0.1 \
    crate://crates.io/bitflags/1.3.2 \
    crate://crates.io/byteorder/1.4.3 \
    crate://crates.io/cc/1.0.71 \
    crate://crates.io/cfg-if/1.0.0 \
    crate://crates.io/chrono/0.4.19 \
    crate://crates.io/clap/2.33.3 \
    crate://crates.io/ctrlc/3.2.1 \
    crate://crates.io/dbus/0.9.5 \
    crate://crates.io/dlv-list/0.2.3 \
    crate://crates.io/epoll/4.3.1 \
    crate://crates.io/fern/0.6.0 \
    crate://crates.io/getrandom/0.2.3 \
    crate://crates.io/glob/0.3.0 \
    crate://crates.io/hashbrown/0.9.1 \
    crate://crates.io/hermit-abi/0.1.19 \
    crate://crates.io/hostname/0.3.1 \
    crate://crates.io/ipnetwork/0.18.0 \
    crate://crates.io/itoa/0.4.8 \
    crate://crates.io/libc/0.2.106 \
    crate://crates.io/libdbus-sys/0.2.2 \
    crate://crates.io/log/0.4.14 \
    crate://crates.io/match_cfg/0.1.0 \
    crate://crates.io/memchr/2.4.1 \
    crate://crates.io/memoffset/0.6.4 \
    crate://crates.io/nix/0.23.0 \
    crate://crates.io/num-integer/0.1.44 \
    crate://crates.io/num-traits/0.2.14 \
    crate://crates.io/ordered-multimap/0.3.1 \
    crate://crates.io/pkg-config/0.3.22 \
    crate://crates.io/pnet/0.28.0 \
    crate://crates.io/pnet_base/0.28.0 \
    crate://crates.io/pnet_datalink/0.28.0 \
    crate://crates.io/pnet_macros/0.28.0 \
    crate://crates.io/pnet_macros_support/0.28.0 \
    crate://crates.io/pnet_packet/0.28.0 \
    crate://crates.io/pnet_sys/0.28.0 \
    crate://crates.io/pnet_transport/0.28.0 \
    crate://crates.io/ppv-lite86/0.2.15 \
    crate://crates.io/proc-macro2/1.0.32 \
    crate://crates.io/quote/1.0.10 \
    crate://crates.io/rand/0.8.4 \
    crate://crates.io/rand_chacha/0.3.1 \
    crate://crates.io/rand_core/0.6.3 \
    crate://crates.io/rand_hc/0.3.1 \
    crate://crates.io/regex/1.5.4 \
    crate://crates.io/regex-syntax/0.6.25 \
    crate://crates.io/rmp/0.8.10 \
    crate://crates.io/rmp-serde/0.15.5 \
    crate://crates.io/rust-ini/0.17.0 \
    crate://crates.io/ryu/1.0.5 \
    crate://crates.io/serde/1.0.130 \
    crate://crates.io/serde_derive/1.0.130 \
    crate://crates.io/serde_json/1.0.68 \
    crate://crates.io/strsim/0.8.0 \
    crate://crates.io/syn/1.0.81 \
    crate://crates.io/textwrap/0.11.0 \
    crate://crates.io/time/0.1.44 \
    crate://crates.io/unicode-width/0.1.9 \
    crate://crates.io/unicode-xid/0.2.2 \
    crate://crates.io/vec_map/0.8.2 \
    crate://crates.io/wasi/0.10.0+wasi-snapshot-preview1 \
    crate://crates.io/winapi/0.3.9 \
    crate://crates.io/winapi-i686-pc-windows-gnu/0.4.0 \
    crate://crates.io/winapi-x86_64-pc-windows-gnu/0.4.0 \
"

LIC_FILES_CHKSUM = " \
    "

HOMEPAGE = "github.com/HULKs/nao"
LICENSE = "CLOSED"

inherit pkgconfig

DEPENDS += "dbus"
RDEPENDS:${PN} += "dbus"

SYSTEMD_PACKAGES = "${PN}"
SYSTEMD_SERVICE:${PN} = "hula.service"

do_install:append() {
  install -d ${D}${systemd_unitdir}/system/
  install -m 0644 ${WORKDIR}/hula.service ${D}${systemd_unitdir}/system/
}

FILES:${PN} += "${systemd_unitdir}/system/hula.service"

inherit systemd
