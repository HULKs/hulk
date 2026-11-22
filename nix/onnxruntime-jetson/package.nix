{ lib, _cuda, cudaPackages_13_2, nativeCuda, callPackage }:
let
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
  passthru = previous.passthru // {
    cudaCompat = callPackage ../cuda-compat-orin/package.nix { };
    l4tRelease = "36.4.3";
  };
})
