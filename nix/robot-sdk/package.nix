{ pkgs, robotPkgs, rustVersion, libraries, onnxruntime }:
let
  target = "aarch64-unknown-linux-gnu";
  compiler = robotPkgs.stdenv.cc;
in
robotPkgs.mkShell {
  strictDeps = true;
  NIX_NO_SELF_RPATH = 1;
  nativeBuildInputs = [
    (pkgs.rust-bin.stable.${rustVersion}.default.override { targets = [ target ]; })
    robotPkgs.buildPackages.pkg-config
    robotPkgs.rustPlatform.bindgenHook
    pkgs.util-linux
  ];
  depsBuildBuild = [ pkgs.stdenv.cc pkgs.pkg-config pkgs.openssl ];
  buildInputs = libraries ++ [ onnxruntime ];

  CARGO_BUILD_TARGET = target;
  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = "${compiler}/bin/${compiler.targetPrefix}cc";
  # CUDA providers and the compatibility driver resolve transitive libraries through this RPATH.
  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS =
    "-Ctarget-cpu=cortex-a78ae -Clink-arg=-Wl,--disable-new-dtags,-rpath,${onnxruntime.cudaCompat}/lib:/usr/lib/aarch64-linux-gnu/tegra";
  ORT_LIB_PATH = "${pkgs.lib.getLib onnxruntime}/lib";
  ORT_PREFER_DYNAMIC_LINK = "1";
  CC_aarch64_unknown_linux_gnu = "${compiler}/bin/${compiler.targetPrefix}cc";
  CXX_aarch64_unknown_linux_gnu = "${compiler}/bin/${compiler.targetPrefix}c++";
  AR_aarch64_unknown_linux_gnu = "${compiler.bintools}/bin/${compiler.targetPrefix}ar";

  # Cargo also compiles native build dependencies inside the cross shell.
  CC_x86_64_unknown_linux_gnu = "${pkgs.stdenv.cc}/bin/cc";
  CXX_x86_64_unknown_linux_gnu = "${pkgs.stdenv.cc}/bin/c++";
  AR_x86_64_unknown_linux_gnu = "${pkgs.stdenv.cc.bintools}/bin/ar";
  X86_64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR = "${pkgs.lib.getLib pkgs.openssl}/lib";
  X86_64_UNKNOWN_LINUX_GNU_OPENSSL_INCLUDE_DIR = "${pkgs.lib.getDev pkgs.openssl}/include";
}
