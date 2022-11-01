# Directory Structure

TODO: Add `crates` directory

TODO: Add `twix`

TODO: Check if up-to-date

The main code repository represents a [monorepo](https://en.wikipedia.org/wiki/Monorepo) containing many parts of the robotic control software and several tools.
The directory structure is organized as follows:

- `.github/`: GitHub Pull Request template and Actions workflow for the Continuous Integration the HULKs are using for development
- `docs/`: This documentation
- `etc/`: All additional files necessary when deploying the code to a robot
    - `configuration/`: Configuration files that are deployed to NAOs and are read during startup
    - `motions/`: Motion files that can be played back on a robot
    - `neural_networks/`: Neural network files for e.g. the ball detection
    - `poses/` and `sounds/`: Legacy files (may be removed at some time)
- `macros/`: Rust sub-crate providing e.g. the [module macros](./macros.md)
- `module_attributes/`: Rust sub-crate providing macro attribute parsing for the [module macros](./macros.md)
- `scripts/`: Legacy scripts and files (may be removed at some time)
- `sdk/`: SDK download directory and version selection symlink
- `spl_network_messages/`: Rust sub-crate providing SPL message parsing including GameController messages
- `src/`: Source code of the robotic control software
    - `audio/`: Audio cycler and all modules that belong to it
    - `behavior_simulator/`: Special runtime that is able to execute a subset of control modules for behavior simulation
    - `bin/`: Code for executables that can be built, e.g. `nao` and `webots` (these contain `main()`)
    - `control/`: Control cycler and all modules that belong to it
    - `framework/`: Code regarding the framework containing e.g. filtering and thread communication primitives or the configuration hierarchy
        - `communication/`: Communication subcomponent containing the interface to the cyclers and all file system and socket I/O
    - `hardware/`: Hardware interface definition and several implementations for the build targets e.g. NAO and Webots, see [Hardware Interface](./hardware_interface.md)
    - `spl_network/`: SPL network cycler and all modules that belong to it
    - `types/`: Rust types that are used throughout the robotics code and framework
    - `vision/`: Vision cycler and all modules that belong to it
- `tests/`: Additional data needed for tests of the robotic control software
- `tools/`: Miscellaneous projects and tools more or less related to the code
    - `ci/`: Dockerfiles and scripts for the Continuous Integration the HULKs are using for development
    - `depp/`: Small tool to create a list of dependencies for Yocto Rust recipies (alternative to `cargo-bitbake`)
    - `fanta/`: Debug client that can attach to communication and dump received data to standard output
    - `flora/`: Graphical debug client that can attach to communication and visualize the current state of the robot
    - `hula/`: Executable which runs on the NAO to provide the HULKs-level abstraction, a wrapper around LoLA, see [Hardware Interface](./hardware_interface.md)
    - `libPython/`: Legacy scripts and files (may be removed at some time)
    - `machine-learning/`: Tooling and training data for machine learning e.g. for ball detection
    - `pepsi/`: Mainly a tool for deploying and interacting with the NAO
    - `sprite/`: Behavior simulator frontend which can visualize the recorded behavior simulation
    - `TextToSpeech/`: Legacy files (may be removed at some time)
- `uvcvideo/`: Rust sub-crate providing Linux USB Video Class driver support for NAO cameras
- `webots/`: Root directory of webots simulation directory structure
