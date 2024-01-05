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
        in
        {
          devShells.default = pkgs.mkShell
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
              ];
              env = {
                LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
                LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
              };
            };
        }
      );
}
