# Compile TensorRT engines on a K1

`tensorrt-compile` is the compiler binary. Pepsi exposes the automation command as
**`tensor-rt-compile`**. Both use the ONNX model's input metadata rather than a
hardcoded detection output name.

Run host commands from the repository root. The robot must be provisioned with a
running `hulk` Podman runtime container and compatible ONNX Runtime, CUDA and
TensorRT libraries. `./pepsi gammaray --help` describes provisioning options.
The model must exist locally under `etc/neural_networks` before upload.

## Model filename and input shape

The detector loads `detection.neural_networks_folder` joined with
`detection.model_name`. The current base model is:

```text
yolo26m-seg=f11+yolo26m~cheek+yolo26m-pose~badge.onnx
```

`hydra-nv12.onnx` is not required by the runtime. For a new export, select its
actual filename in the intended detection parameter layer.
See `tools/machine-learning/multi-task-yolo/REPRODUCE.md` for checkpoint and export
setup, including the distinction between the historical recipe and current model.

Dynamic input dimensions require an explicit shape for **every dynamic input**.
Append `--<input-name> dim1,dim2,...` after the model argument when invoking the
binary, or after `--` when forwarding through Pepsi. Both separated and
`--<input-name>=dim1,dim2,...` forms are accepted. Dimensions must be positive,
match the tensor rank and agree with any fixed model dimensions.

For the dynamic NV12 input `raw_bytes_input`, a 640×544 image uses `272,320,6`.
Choose the size used by the deployed image pipeline; the Hydra export trace's
default square size is not the deployment size. Static models, including the fused
XFeat/LighterGlue export, resolve shapes from metadata and need no overrides.

## Automated workflow

For the current configured model and robot 42:

```bash
MODEL='yolo26m-seg=f11+yolo26m~cheek+yolo26m-pose~badge.onnx'
./pepsi hulk stop 42
./pepsi tensor-rt-compile "etc/neural_networks/$MODEL" 42 -- --raw_bytes_input 272,320,6
```

Pepsi builds the compiler using its manifest's default K1 SDK environment, uploads
the compiler and `etc`, runs it inside the existing `hulk` container, and downloads
the neural-network directory. Stop HULK first: this upload uses clean synchronization
and can replace/remove existing remote files, including binaries absent from the
compiler upload directory. It does not itself stop or restart the HULK service.
Keep the runtime container running; stopping the HULK application service is
different from killing that container.

Use `--no-build` only with a compiler already built for the same environment,
profile and target directory. Build overrides precede the forwarding `--`, for
example `--profile with-debug` or `--env podman`.

After successful compilation, restore/deploy the normal robot stack and cache:

```bash
./pepsi upload 42
```

Upload includes `etc/neural_networks` and restarts HULK. Cache reuse requires a
compatible model, runtime, target GPU and input profile; the presence of files alone
does not guarantee that runtime compilation will be skipped.

## Manual workflow

Build the compiler and upload the model/configuration while leaving HULK stopped:

```bash
./pepsi build tools/tensorrt-compile
./pepsi upload 42 --no-restart
rsync -av target/container/aarch64-unknown-linux-gnu/debug/tensorrt-compile \
  booster@10.1.24.42:~/hulk/bin/tensorrt-compile
./pepsi shell 42
```

The positional manifest selects cross-compilation by default. If you changed the
environment, profile or `--target-dir`, use the matching artifact path for `rsync`.

On the robot, run in the provisioned runtime container:

```bash
sudo podman exec --user "$(id -u booster)" hulk ./bin/tensorrt-compile \
  --cache-path /home/booster/hulk/etc/neural_networks \
  /home/booster/hulk/etc/neural_networks/yolo26m-seg=f11+yolo26m~cheek+yolo26m-pose~badge.onnx \
  --raw_bytes_input 272,320,6
```

The provisioned container has working directory `/home/booster/hulk` and mounts
the host home directory. Inspect `tools/k1-setup/hulk-runtime.container` if your
installation differs. `launchHULK --executable ...` is not the current repository
launcher interface.

Back on the host, retrieve generated cache files and deploy:

```bash
rsync -av 'booster@10.1.24.42:~/hulk/etc/neural_networks/*Tensorrt*' etc/neural_networks/
./pepsi upload 42
```

## Success and failures

The compiler creates a TensorRT-backed session, allocates dummy inputs from model
metadata, and runs inference. A zero exit status is the success criterion; Pepsi
rejects a nonzero compiler exit. Do not treat an arbitrary error as successful
compilation even if partial cache files exist. Dynamic-shape errors require shape
overrides; missing runtime/provider libraries require fixing the target environment.
