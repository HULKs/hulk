# Bauen der HULKs CrossToolchain

## Anleitung

 0. Required packages  : build-essential automake autoconf gperf bison flex texinfo libtool libtool-bin gawk libncurses5-dev unzip cmake libexpat-dev python2.7-dev nasm help2man ninja

    Required variables : LD_LIBRARY_PATH and CPATH need to be empty

    Note: ubuntu users might have a different ninja package preinstalled which is not the required one, if errors occur do `sudo apt-get purge ninja` and `sudo apt-get install ninja-build`
 1. Das Script `1-setup` ausführen.
 2. Das Script `2-build-toolchain` ausführen.
 3. Prüfen, ob die toolchain richtig gebaut wurde (in x-tools nachgucken, ob dort die toolchain existiert)
 4. Das Script `3-build-libs` ausführen.
 5. Das Script `4-install` ausführen.
 6. Es sollte einen Ordner ctc-linux**-hulks-** geben.
 7. Es sollten ein sysroot und ctc-linux**-hulks-** tar.gz geben

## Known issues

 * The nao has a 2.6 kernel. Glibc > 2.23 needs kernel >= 3.2.

