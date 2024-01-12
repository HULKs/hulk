{
  description = "Dev environment for HULKs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    nixgl.url = "github:guibou/nixGL";
  };

  outputs = { self, nixpkgs, flake-utils, nixgl }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ nixgl.overlay ];
          };

          nao_sdk_version = "5.9.0";
          nao_sdk_environment_path = "$HOME/.naosdk/${nao_sdk_version}/environment-setup-corei7-64-aldebaran-linux";
        in
        {
          devShells = {
            tools = pkgs.mkShell
              rec {
                buildInputs = with pkgs;[
                  # Tools
                  cargo
                  rustc
                  rustfmt
                  cmake
                  pkg-config
                  llvmPackages.clang
                  python312
                  rsync

                  # Libs
                  luajit
                  systemdLibs
                  hdf5
                  alsa-lib
                  opusfile
                  libogg
                  libGL
                  libxkbcommon
                  wayland
                  xorg.libX11
                  xorg.libXcursor
                  xorg.libXi
                  xorg.libXrandr
                  pkgs.nixgl.auto.nixGLDefault
                  rustPlatform.bindgenHook
                ];
                env = {
                  LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
                  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
                };
              };


            robot = (pkgs.buildFHSUserEnv {
              name = "hulk-robot-dev-env";
              targetPkgs = pkgs: ([]);
              extraOutputsToInstall = [ "dev" ];
              runScript = "bash";
              profile = ''
                if [[ ! -f "${nao_sdk_environment_path}" ]]; then
                  echo "WARNING: nao sdk v${nao_sdk_version} not found! Please install it."
                  exit 1
                fi
                echo "Unsetting LD_LIBRARY_PATH..."
                unset LD_LIBRARY_PATH
                echo "Sourcing nao sdk v${nao_sdk_version}..."
                source ${nao_sdk_environment_path}
              '';
            }).env;
          };
        }
      );
}
