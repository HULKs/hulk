{
  description = "Development environment and tools for HULKs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { nixpkgs, crane, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      craneLibrary = crane.mkLib pkgs;

      src = pkgs.lib.fileset.toSource {
        root = ./.;
        fileset = pkgs.lib.fileset.unions [
          (craneLibrary.fileset.commonCargoSources ./.)
          ./crates/hsl_network_messages/headers/RoboCupGameControlData.hpp
          ./tools/pepsi/src/install-podman.sh
        ];
      };

      guiLibraries = with pkgs; [ libGL libxkbcommon wayland libx11 ];
      workspaceVersion =
        (fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;

      mkCrate = {
        name,
        cargoToml,
        package ? name,
        nativeBuildInputs ? [ ],
        libraries ? [ ],
      }:
        let
          packageVersion =
            (fromTOML (builtins.readFile cargoToml)).package.version;
          version = if builtins.isString packageVersion then packageVersion else workspaceVersion;
        in
        craneLibrary.buildPackage {
          inherit src version;
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
      packages.${system} = { inherit pepsi twix rosz; };

      devShells.${system}.default = craneLibrary.devShell {
        inputsFrom = [ pepsi twix rosz ];
        packages = with pkgs; [ rust-analyzer rsync openssh ];
        env.LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath guiLibraries;
      };
    };
}
