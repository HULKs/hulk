{ pkgs, robotPkgs, rustVersion, libraries, onnxruntime }:
let
  target = "aarch64-unknown-linux-gnu";
  compiler = robotPkgs.stdenv.cc;
  rustToolchain =
    (pkgs.rust-bin.stable.${rustVersion}.default.override { targets = [ target ]; }).overrideAttrs
      (previous: {
        # The native compiler belongs in depsBuildBuild, not the toolchain's propagated host role.
        propagatedBuildInputs = pkgs.lib.remove pkgs.stdenv.cc (previous.propagatedBuildInputs or [ ]);
      });
in
robotPkgs.mkShell {
  strictDeps = true;
  NIX_NO_SELF_RPATH = 1;
  nativeBuildInputs = [
    rustToolchain
    robotPkgs.buildPackages.pkg-config
    robotPkgs.rustPlatform.bindgenHook
    pkgs.util-linux
  ];
  depsBuildBuild = [ pkgs.stdenv.cc pkgs.pkg-config pkgs.openssl ];
  # ort-sys uses pregenerated bindings and links the library shipped in the runtime bundle.
  buildInputs = libraries ++ [ onnxruntime.out ];

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
  CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = "${pkgs.stdenv.cc}/bin/cc";
  CC_x86_64_unknown_linux_gnu = "${pkgs.stdenv.cc}/bin/cc";
  CXX_x86_64_unknown_linux_gnu = "${pkgs.stdenv.cc}/bin/c++";
  AR_x86_64_unknown_linux_gnu = "${pkgs.stdenv.cc.bintools}/bin/ar";
  X86_64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR = "${pkgs.lib.getLib pkgs.openssl}/lib";
  X86_64_UNKNOWN_LINUX_GNU_OPENSSL_INCLUDE_DIR = "${pkgs.lib.getDev pkgs.openssl}/include";

  shellHook = ''
    # nix develop preserves inherited variables the SDK doesn't set itself.
    if [ "''${HULK_NATIVE_SHELL:-}" = 1 ]; then
      unset HULK_NATIVE_SHELL CARGO_ENCODED_RUSTFLAGS NIX_CFLAGS_LINK ORT_SKIP_DOWNLOAD
    fi

    if [ -n "''${HULK_ROBOT_BUILD_ROOT:-}" ]; then
      if ! robot_build_root="$(realpath -e -- "$HULK_ROBOT_BUILD_ROOT")"; then
        printf 'Robot SDK: cannot resolve HULK_ROBOT_BUILD_ROOT: %s\n' "$HULK_ROBOT_BUILD_ROOT" >&2
        exit 1
      fi
      export CARGO_TARGET_DIR="$robot_build_root/target/robot"
      export CARGO_HOME="''${XDG_DATA_HOME:-$HOME/.local/share}/hulks/robot-cargo-home"
      unset robot_build_root HULK_ROBOT_BUILD_ROOT
    fi
  '';
}
