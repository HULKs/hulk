# Process Entrypoint

The HULKs robotic control software can be compiled for multiple build targets e.g. NAO and Webots.
Each build target results in a executable which is either executed directly on the NAO or on the development machine.
All executables define a `main()` function as entrypoint for the robotic control software, see `crates/hulks_nao` or `crates/hulks_webots` in the code.
The following sections explain the first setup steps done in the `main()` function for the major build targets NAO and Webots.

## Hardware Parameters

Later in the `main()`, the hardware interfaces are created.
Beside the robotics domain, the hardware interface also needs some configuration parameters to initialize the hardware.
Thes parameters are read from a JSON file that can be passed as first command line argument to the executable.
If omitted, the file at `etc/parameters/hardware.json` is loaded.

## Shutdown and CancellationToken

The `main()` functions for the NAO and Webots targets register shutdown handlers via the [Rust crate ctrlc](https://docs.rs/ctrlc/latest/ctrlc/).
These shutdown handlers react on the Linux signals `SIGINT` and `SIGTERM` to call [`CancellationToken::cancelled()`](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html#method.cancel) which cancels the `CancellationToken` on signal receival.
The `CancellationToken` is a synchronization primitive which is shared with the whole framework and robotics code to allow to shutdown all subcomponents from any location.
Several places listen for the `cancelled()` event and terminate on cancellation.
Beside cancelling the `CancellationToken` on Linux signals, error conditions within the robotic control software can trigger a cancellation as well.
This concept allows to shutdown gracefully in any case of error or termination request.

## Hardware Interface & Runtime

On NAO and in Webots the robotic control software needs access to the hardware or simulator interface.
The hardware interface provides an abstract way to interact with the underlying backend.
The `main()` function first initializes the hardware interface and then starts the runtime (`run()`) with it.
See [Hardware Interface](./hardware_interface.md) for more information about what the hardware interface initializes.
