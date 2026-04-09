# TensorRT compile guide (Hydra NV12 ONNX)

This guide describes the host + robot workflow for compiling TensorRT engines
for a Hydra ONNX model and syncing the compiled cache back into the repository.

The example model can be produced with
[`tools/machine-learning/multi-task-yolo/REPRODUCE.md`](../machine-learning/multi-task-yolo/REPRODUCE.md).

## Workflow overview

1. Cross-compile `tensorrt-compile` on the development host via `pepsi`.
2. `pepsi upload` to sync model file.
3. Sync the binary to the robot.
4. Run compilation on the robot.
5. Sync generated TensorRT cache files back to the development host.
6. Deploy with `pepsi upload` so precompiled cache is shipped with the model.

## Step 1: Build `tensorrt-compile` on host

Run from repository root:

```bash
./pepsi build tools/tensorrt-compile
```

This uses the cross-compilation environment and produces an aarch64 binary
under `target/aarch64-unknown-linux-gnu/debug/`.

## Step 2: Upload to sync model file to robot

```bash
./pepsi upload <ROBOT_NUMBER_OR_IP>
```

## Step 3: Sync binary to robot

Copy the binary to the robot:

```bash
rsync -av target/aarch64-unknown-linux-gnu/debug/tensorrt-compile \
  booster@<robot-ip>:~/hulk/bin/tensorrt-compile
```

## Step 4: Run compilation on robot

Open a shell to the robot:

```bash
./pepsi shell <ROBOT_NUMBER_OR_IP>
```

Then ensure the ONNX model exists in:

- `~/hulk/etc/neural_networks`

Stop hulk process, if it is already running:

```bash
hulk stop
```

If hulk is still running after that:

```bash
sudo podman kill hulk
```

Start compilation:

```bash
launchHULK --executable ~/hulk/bin/tensorrt-compile \
  --onnx-model /home/booster/hulk/etc/neural_networks/hydra-nv12.onnx \
  --cache-path /home/booster/hulk/etc/neural_networks
```

The process may fail with an error, if the output name of the network has changed.
The compilation will still have succeded, if the error occurs.

## Step 5: Sync compiled TensorRT cache back

Copy generated engine/profile files from robot back into the repository:

```bash
rsync -av booster@<robot-ip>:~/hulk/etc/neural_networks/*Tensorrt* \
  etc/neural_networks
```

## Step 6: Deploy with precompiled cache

Now deploy as usual:

```bash
./pepsi upload <ROBOT_NUMBER_OR_IP>
```

Because `etc/neural_networks` now contains compiled TensorRT cache files,
`pepsi upload` syncs them to the robot and HULK can start without waiting for
first-run engine compilation.

## Notes

- `hydra-nv12.onnx` should stay in `etc/neural_networks` with that name unless
  runtime code is changed.
