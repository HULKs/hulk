# Inference runtime compatibility

The robot uses ONNX Runtime 1.22, CUDA 12.8, TensorRT 10.7, and cuDNN 9.7
from the existing container image. The Rust workspace uses `ort` rc.13 with
`api-21`. Do not enable its default features or `api-22` and newer without
revalidating the runtime. Automatic device selection enabled by `api-22`
aborted in the ONNX Runtime 1.22 CPU build during local validation.

Use `GraphOptimizationLevel::All` to retain rc.10's `Level3` behavior.
In rc.13, `Level3` selects `ORT_ENABLE_LAYOUT`, which ONNX Runtime 1.22
rejects with `graph_optimization_level is not valid`.

`hulk-runtime.container` sets both `ORT_DYLIB_PATH` and `LD_PRELOAD` to
`/usr/local/lib/libonnxruntime.so`. Preloading is required for the rc.13
shutdown order: its environment must be released before ONNX Runtime's C++
destructors. Loading only through `ORT_DYLIB_PATH` produced a heap-corruption
abort on process exit with the Linux x64 ONNX Runtime 1.22 build, even with
only a session builder and no inference.

`pepsi gammaray` uploads the container definition before reloading systemd,
stops HULK, restarts `hulk-runtime.service` to recreate the container, and
then starts HULK. Run this setup before deploying the upgraded binaries.
This does not require a new image or a CUDA upgrade. Verify startup and
shutdown on the robot, including GPU provider selection and model inference,
before rollout. Local CPU validation does not cover Jetson CUDA or TensorRT
execution.

For local dynamic-loading runs, set both variables to the same absolute
library path. The opt-in regression test runs a Float32 Identity model with
all graph optimizations, contiguous and transposed ndarray inputs, and
process cleanup:

```bash
ORT_DYLIB_PATH=/absolute/path/to/libonnxruntime.so \
LD_PRELOAD=/absolute/path/to/libonnxruntime.so \
cargo test --locked -p hydra-bench --test runtime -- --ignored
```

The test fixture uses ONNX IR 8, opset 13, input `images`, output `out`, and
shape `[1, 3, 2, 2]`. No GPU is needed for this test.
