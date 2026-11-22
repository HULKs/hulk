{ lib, buildEnv, robotPkgs, robotBridge, onnxruntime, version, libraries, rootfsOverlay }:
let
  profilingPerf = robotPkgs.perf.overrideAttrs (previous: {
    postPatch = previous.postPatch + ''
      # This robot has a Nix profile, not a NixOS system debug directory.
      substituteInPlace util/build-id.c util/dso.c \
        --replace-fail /run/current-system/sw/lib/debug ${robotPkgs.glibc.debug}/lib/debug
    '';
  });
  metadata = builtins.toJSON {
    runtime_version = version;
    system = "aarch64-linux";
    l4t = onnxruntime.l4tRelease;
  };
in
buildEnv {
  name = "hulk-runtime";
  paths = with robotPkgs; [ zenoh robotBridge nftables patchelf sshpass profilingPerf glibc.debug ];
  pathsToLink = [ "/bin" "/lib/debug" ];
  postBuild = ''
    # nix store cat requires a regular file rather than a buildEnv symlink.
    mkdir -p "$out/share/hulk-runtime"
    printf '%s' ${lib.escapeShellArg metadata} > "$out/share/hulk-runtime/runtime.json"
    cp -r ${rootfsOverlay} "$out/share/hulk-runtime/rootfs_overlay"
    # Retain the libraries referenced by uploaded ELF binaries in the offline closure.
    mkdir "$out/share/hulk-runtime/libraries"
    ln -s ${lib.escapeShellArgs (map lib.getLib ([ onnxruntime onnxruntime.cudaCompat ] ++ libraries))} \
      "$out/share/hulk-runtime/libraries/"
    ln -s ${lib.getLib onnxruntime}/lib "$out/share/hulk-runtime/onnxruntime"
  '';
}
