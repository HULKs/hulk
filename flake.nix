{
  description = "Development environment and tools for HULKs";

  nixConfig = {
    fallback = true;
    extra-substituters = [ "https://nix-cache.hulks.dev/hulk" ];
    extra-trusted-public-keys = [
      "hulk:724ZyCrz/G8uthHJmwTxfsk3qNDHArarIm6yFyU8t9I="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { nixpkgs, crane, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      workspace = fromTOML (builtins.readFile ./Cargo.toml);
      rustVersion = workspace.workspace.package.rust-version;
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
        config.allowUnfree = true;
      };
      robotPkgs = import nixpkgs {
        localSystem = system;
        crossSystem = "aarch64-linux";
        config = {
          allowUnfree = true;
          cudaForwardCompat = false;
        };
      };
      robotBridge = pkgs.pkgsCross.aarch64-multiplatform.callPackage ./nix/zenoh-bridge-dds/package.nix { };
      robotOnnxruntime = robotPkgs.callPackage ./nix/onnxruntime-jetson/package.nix {
        nativeCuda = pkgs.cudaPackages_13_2;
      };
      robotLibraries = robotPkgs.callPackage ./nix/robot-dependencies.nix { };
      rustToolchain = pkgs.rust-bin.stable.${rustVersion}.default;
      craneLibrary =
        (crane.mkLib pkgs).overrideToolchain rustToolchain;
      nativeStdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;
      nativeOrt = "${pkgs.lib.getLib pkgs.onnxruntime}/lib";

      workspaceFiles = pkgs.lib.fileset.unions [
        (craneLibrary.fileset.commonCargoSources ./.)
        ./crates/hsl_network_messages/headers/RoboCupGameControlData.hpp
        ./tools/pepsi/src/install-hulks-bundle.sh
        ./tools/pepsi/src/gammaray
      ];

      guiLibraries = with pkgs; [ libGL libxkbcommon wayland libx11 vulkan-loader ];
      workspaceVersion = workspace.workspace.package.version;

      mkCrate = {
        name,
        cargoToml,
        package ? name,
        extraFiles ? [ ],
        nativeBuildInputs ? [ ],
        libraries ? [ ],
      }:
        let
          packageVersion =
            (fromTOML (builtins.readFile cargoToml)).package.version;
          version = if builtins.isString packageVersion then packageVersion else workspaceVersion;
        in
        craneLibrary.buildPackage {
          inherit version;
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions ([ workspaceFiles ] ++ extraFiles);
          };
          pname = name;
          strictDeps = true;
          nativeBuildInputs =
            nativeBuildInputs ++ pkgs.lib.optional (libraries != [ ]) pkgs.makeWrapper;
          buildInputs = libraries;
          cargoExtraArgs = "--locked -p ${package} --bin ${name}";
          doCheck = false;
          postInstall = pkgs.lib.optionalString (libraries != [ ]) ''
            wrapProgram "$out/bin/${name}" \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath libraries}
          '';
        };

      pepsi = mkCrate {
        name = "pepsi";
        cargoToml = ./tools/pepsi/Cargo.toml;
      };

      twix = mkCrate {
        name = "twix";
        cargoToml = ./tools/twix/Cargo.toml;
        extraFiles = [ ./tools/twix/presets ];
        nativeBuildInputs = with pkgs; [ pkg-config rustPlatform.bindgenHook ];
        libraries = guiLibraries;
      };

      rosz = mkCrate {
        name = "rosz";
        package = "ros-z-cli";
        cargoToml = ./crates/ros-z-cli/Cargo.toml;
      };
    in
    {
      packages.${system} = {
        inherit pepsi twix rosz;
        zenoh-bridge-dds = pkgs.callPackage ./nix/zenoh-bridge-dds/package.nix { };
      };

      packages.aarch64-linux.zenoh-bridge-dds = robotBridge;

      packages.aarch64-linux.onnxruntime-jetson = robotOnnxruntime;

      packages.aarch64-linux.hulk-runtime = pkgs.callPackage ./nix/hulk-runtime/package.nix {
        inherit robotPkgs robotBridge;
        libraries = robotLibraries;
        onnxruntime = robotOnnxruntime;
        rootfsOverlay = ./tools/k1-setup/rootfs_overlay;
        version = (fromTOML (builtins.readFile ./hulk.toml)).runtime_version;
      };

      devShells.${system} = {
        default = (pkgs.mkShell.override { stdenv = nativeStdenv; }) {
          strictDeps = true;
          nativeBuildInputs = with pkgs; [
            rustToolchain
            rust-analyzer
            cargo-nextest
            taplo
            ruff
            patchelf
            pkg-config
            rustPlatform.bindgenHook
            rsync
            openssh
          ];
          buildInputs = with pkgs; guiLibraries ++ [
            alsa-lib
            fontconfig
            mesa
            onnxruntime
            openssl
            systemdLibs
          ];
          env = {
            HULK_NATIVE_SHELL = "1";
            NIX_NO_SELF_RPATH = "1";
            CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = "${nativeStdenv.cc}/bin/cc";
            # Use the wrapper's mold, not rustc's bundled LLD or a distro linker from Cargo config.
            CARGO_ENCODED_RUSTFLAGS = "-Clink-arg=-fuse-ld=mold";
            ORT_LIB_PATH = nativeOrt;
            ORT_PREFER_DYNAMIC_LINK = "1";
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath guiLibraries;
          };
        };
        android =
          let
            ndk = (pkgs.androidenv.composeAndroidPackages {
              includeNDK = true;
              ndkVersion = "27.3.13750724";
            }).ndk-bundle;
            ndkBin = "${ndk}/libexec/android-sdk/ndk-bundle/toolchains/llvm/prebuilt/linux-x86_64/bin";
            linker = "${ndkBin}/aarch64-linux-android21-clang";
          in
          pkgs.mkShell {
            nativeBuildInputs = [
              (rustToolchain.override { targets = [ "aarch64-linux-android" ]; })
            ];
            env = {
              CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = linker;
              CC_aarch64_linux_android = linker;
              AR_aarch64_linux_android = "${ndkBin}/llvm-ar";
            };
          };
        robot-sdk = import ./nix/robot-sdk/package.nix {
          inherit pkgs robotPkgs rustVersion;
          libraries = robotLibraries;
          onnxruntime = robotOnnxruntime;
        };
      };
    };
}
