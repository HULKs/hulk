{ lib, _cuda, cudaPackages_13_2, nativeCuda, callPackage, patchelf }:
let
  cudaCompat = callPackage ../cuda-compat-orin/package.nix { };
  cuda = (cudaPackages_13_2.override {
    manifests = cudaPackages_13_2.manifests // {
      cudnn = _cuda.manifests.cudnn."9.13.0";
      tensorrt = _cuda.manifests.tensorrt."10.16.1";
    };
  }).overrideScope (final: previous: {
    # Run NVCC on the build host, but compile its CPU code with the ARM toolchain.
    cuda_nvcc = nativeCuda.cuda_nvcc.override {
      inherit (final) setupCudaHook;
    };
    cudnn-frontend = previous.cudnn-frontend.override {
      cuda_nvcc = final.cuda_nvcc;
      withSamples = false;
      withTests = false;
    };
  });
in
(cuda.pkgs.onnxruntime.override {
  cudaPackages = cuda;
  cudaSupport = true;
  pythonSupport = false;
  openvinoSupport = false;
  ncclSupport = false;
}).overrideAttrs (previous: {
  nativeBuildInputs = previous.nativeBuildInputs ++ [ patchelf ];
  buildInputs = previous.buildInputs ++ [ cuda.tensorrt ];
  cmakeFlags = previous.cmakeFlags ++ [
    (lib.cmakeFeature "CMAKE_CUDA_ARCHITECTURES" "87-real")
    (lib.cmakeBool "onnxruntime_USE_TENSORRT" true)
    (lib.cmakeBool "onnxruntime_USE_TENSORRT_BUILTIN_PARSER" true)
  ];
  postInstall = previous.postInstall + ''
    test -f "$out/lib/libonnxruntime_providers_cuda.so"
    test -f "$out/lib/libonnxruntime_providers_tensorrt.so"
  '';
  postFixup = (previous.postFixup or "") + ''
    # The provider's RUNPATH, not the executable's RPATH, resolves libcuda.so.1.
    provider="$out/lib/libonnxruntime_providers_cuda.so"
    patchelf --add-rpath '${cudaCompat}/lib' "$provider"
    case ":$(patchelf --print-rpath "$provider"):" in
      *':${cudaCompat}/lib:'*) ;;
      *) echo "CUDA provider is missing the Orin compatibility library path" >&2; exit 1 ;;
    esac

    # TensorRT can initialize CUDA before the CUDA provider loads the driver.
    provider="$out/lib/libonnxruntime_providers_tensorrt.so"
    rpath=$(patchelf --print-rpath "$provider")
    patchelf --add-needed libcuda.so.1 \
      --set-rpath "${cudaCompat}/lib''${rpath:+:$rpath}" "$provider"
    patchelf --print-needed "$provider" | grep -Fx libcuda.so.1
  '';
  passthru = previous.passthru // {
    inherit cudaCompat;
    l4tRelease = "36.4.3";
  };
})
