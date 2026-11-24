{ lib, stdenvNoCC, fetchurl, dpkg, patchelf, binutils }:
stdenvNoCC.mkDerivation {
  pname = "cuda-compat-orin";
  version = "13.2.44290101";
  src = fetchurl {
    url = "https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/sbsa/cuda-compat-orin-13-2_13.2.44290101-1_arm64.deb";
    sha256 = "3331f3ac94eecc0e3c0d1976c8fa306c2bf33dd6f771e0d61bb583cab3590037";
  };
  nativeBuildInputs = [ dpkg patchelf binutils ];
  dontUnpack = true;
  dontBuild = true;
  installPhase = ''
    runHook preInstall
    dpkg-deb --extract "$src" unpacked
    mkdir -p "$out/lib" "$out/share/doc"
    cp -a unpacked/usr/local/cuda-13.2/compat_orin/lib*.so* "$out/lib/"
    cp -a unpacked/usr/share/doc/cuda-compat-orin-13-2 "$out/share/doc/"
    runHook postInstall
  '';
  postFixup = ''
    # Tegra libraries such as libnvrm_gpu.so also need to find libnvos.so.
    driver=$(readlink -f "$out/lib/libcuda.so.1")
    patchelf --force-rpath --add-rpath '$ORIGIN:/usr/lib/aarch64-linux-gnu/tegra' "$driver"
    "$READELF" -d "$driver" | grep -E '\(RPATH\).*/usr/lib/aarch64-linux-gnu/tegra'
  '';
  meta = {
    description = "Orin CUDA 13.2 userspace driver without a BSP upgrade";
    license = lib.licenses.nvidiaCuda;
    platforms = [ "aarch64-linux" ];
  };
}
