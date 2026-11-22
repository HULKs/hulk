{ glibc, stdenv, alsa-lib, systemdLibs, libcap, zlib, openssl }:
[ glibc stdenv.cc.cc.lib alsa-lib systemdLibs libcap zlib openssl ]
