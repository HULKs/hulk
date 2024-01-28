{
  description = "Dev environment for HULKs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixgl.url = "github:guibou/nixGL";
  };

  outputs = { self, nixpkgs, flake-utils, naersk, nixgl }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ nixgl.overlay ];
          };
          naersk' = pkgs.callPackage naersk { };
          nao_sdk_version = "5.9.0";
          nao_sdk_environment_path = "$HOME/.naosdk/${nao_sdk_version}/environment-setup-corei7-64-aldebaran-linux";
          buildInputs = with pkgs;[
            # Tools
            cargo
            cmake
            llvmPackages.clang
            pkg-config
            python312
            rsync
            rustc
            rustfmt

            # Libs
            alsa-lib
            hdf5
            libGL
            libogg
            libxkbcommon
            luajit
            openssl
            opusfile
            pkgs.nixgl.auto.nixGLDefault
            rustPlatform.bindgenHook
            systemdLibs
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
          ];

          mktool = manifest_path:
            let
              manifest = (pkgs.lib.importTOML manifest_path).package;
            in
            naersk'.buildPackage {
              name = manifest.name;
              version = manifest.version;
              src = ./.;
              inherit buildInputs;
              cargoBuildOptions = x: x ++ [ "-p" manifest.name ];
              cargoTestOptions = x: x ++ [ "-p" manifest.name ];
            };

          mktool_wrapper_gui = tool:
            let
              binary_name = pkgs.lib.strings.removeSuffix "-${tool.version}" tool.name;
              wrapper = pkgs.writeShellScriptBin "${tool.name}-wrapper" ''
                ${pkgs.nixgl.auto.nixGLDefault}/bin/nixGL ${tool}/bin/${binary_name} $@
              '';
            in
            {
              type = "app";
              program = "${wrapper}/bin/${tool.name}-wrapper";
            };
        in
        {
          packages.twix = mktool ./tools/twix/Cargo.toml;
          packages.pepsi = mktool ./tools/pepsi/Cargo.toml;
          packages.fanta = mktool ./tools/fanta/Cargo.toml;
          packages.annotato = mktool ./tools/annotato/Cargo.toml;
          packages.behavior_simulator = mktool ./tools/behavior_simulator/Cargo.toml;

          # Needed for non-nixos systems
          apps.twix = mktool_wrapper_gui self.packages.${system}.twix;

          # Needed for non-nixos systems
          apps.annotato  = mktool_wrapper_gui self.packages.${system}.annotato;

          devShells = {
            tools = pkgs.mkShell
              rec {
                inherit buildInputs;
                env = {
                  LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
                  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
                };
              };

            robot = (pkgs.buildFHSUserEnv {
              name = "hulk-robot-dev-env";
              targetPkgs = pkgs: ([ ]);
              extraOutputsToInstall = [ "dev" ];
              runScript = "bash";
              profile = ''
                if [[ ! -f "${nao_sdk_environment_path}" ]]; then
                  echo "ERROR: nao sdk v${nao_sdk_version} not found! Please install it."
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
