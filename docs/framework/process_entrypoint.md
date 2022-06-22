# Process Entrypoint

The HULKs robotic control software can be compiled for multiple build targets e.g. NAO and Webots. Each build target results in a executable which is either executed directly on the NAO or on the development machine. All executables define a `main()` function as entrypoint for the robotic control software, see `src/bin/` in the code. The following sections explain the first setup steps done in the `main()` function for the major build targets NAO and Webots. The final sections cover the behavior simulator entrypoint briefly.

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
The `main()` function first initializes the hardware interface and then constructs the runtime with it.
See [Hardware Interface](./hardware_interface.md) for more information about what the hardware interface initializes.
At the end, the runtime is started.
The `main()` function then waits for termination of the runtime which then concludes the process execution.

## Behavior Simulator

The behavior simulator is a special build target which only initializes and starts a subset of the robotic control software.
It is intended to be executed on the development machine.
Cancellation and hardware interfaces are not needed and are therefore omitted from initialization in `main()`.
Instead, the behavior simulator parses command line arguments and dispatches the behavior simulation.
