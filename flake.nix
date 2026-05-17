{
  description = "Dev Environment for HULKs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        craneLibrary = crane.mkLib pkgs;

        src = pkgs.lib.cleanSourceWith {
          src = craneLibrary.path ./.;
          filter = path: type:
            (builtins.match ".*/(crates|tools)/.*" path != null) ||
            (craneLibrary.filterCargoSources path type);
        };

        baseNativeBuildInputs = with pkgs; [ pkg-config rustPlatform.bindgenHook ];
        baseBuildInputs = with pkgs; [ openssl ];
        guiLibraries = with pkgs; [ libGL libxkbcommon wayland libx11 udev ];

        workspaceCargoToml = fromTOML (builtins.readFile ./Cargo.toml);
        workspaceVersion = workspaceCargoToml.workspace.package.version;

        mkCrate = { name, package ? name, cargoToml, extraLibraries ? [] }:
          let
            localCargoToml = fromTOML (builtins.readFile cargoToml);

            # Crane currently cannot resolve `version = { workspace = true }`.
            # Resolve the version by checking the local Cargo.toml first, then falling back to the workspace root
            resolvedVersion =
              if localCargoToml.package ? version && builtins.isString localCargoToml.package.version then
                localCargoToml.package.version
              else
                workspaceVersion;

            arguments = {
              inherit src;
              pname = name;
              version = resolvedVersion;
              nativeBuildInputs = baseNativeBuildInputs ++ pkgs.lib.optional (extraLibraries != []) pkgs.makeWrapper;
              buildInputs = baseBuildInputs ++ extraLibraries;
              cargoExtraArgs = "-p ${package} --bin ${name}";
            };

            cargoArtifacts = craneLibrary.buildDepsOnly arguments;
          in
          craneLibrary.buildPackage (arguments // {
            inherit cargoArtifacts;
            postInstall = pkgs.lib.optionalString (extraLibraries != [])  ''
              if [ -f $out/bin/${name} ]; then
                wrapProgram $out/bin/${name} \
                  --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath extraLibraries}
              fi
            '';
          });
      in
      {
        packages = {
          pepsi = mkCrate {
            name = "pepsi";
            cargoToml = ./tools/pepsi/Cargo.toml;
          };

          twix = mkCrate {
            name = "twix";
            cargoToml = ./tools/twix/Cargo.toml;
            extraLibraries = guiLibraries;
          };

          rosz = mkCrate {
            name = "rosz";
            package = "ros-z-cli";
            cargoToml = ./crates/ros-z-cli/Cargo.toml;
          };

          default = self.packages.${system}.pepsi;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [
            self.packages.${system}.pepsi
            self.packages.${system}.twix
            self.packages.${system}.rosz
          ];

          packages = with pkgs; [
            cargo
            rustc
            rust-analyzer
            rustfmt
            clippy
            rsync
            openssh
          ];

          env = {
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath guiLibraries;
          };
        };
      }
    );
}
