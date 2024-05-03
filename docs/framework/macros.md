# Macros

TODO: Elaborate

- Macros
    - What is a Rust macro? Gets a TokenStream as input, is able to transform it and outputs a new TokenStream
    - Goal: Reduce code duplication, reduce manually-written code
    - Node
        - Node declaration `#[node(...)]` Declare a node
            - Attached to `impl Node {}`
            - Add `struct CycleContext`
                - Contains inputs, additional outputs, etc.
            - Add `impl CycleContext { fn new(...) -> CycleContext {} }`
            - Add `struct MainOutputs`
                - Contains main outputs
            - Add `impl MainOutputs { fn update(...) {} fn none() {} }`
            - Modify `impl Node {}`: Add `fn run_cycle() {}`
                - Creates `CycleContext` and `MainOutputs`
                - Call `cycle()` method of the node
        - Inputs
            - Input `#[input(path, data_type, cycler, name)]` Get data from this cycle within the current cycler
            - Within control cycler:
                - Historic Input `#[historic_input(path, data_type, name)]` Get historic data from control cycler
                - Perception Input `#[perception_input(path, data_type, cycler, name)]` Get perception data from perception cyclers
                - Persistent State `#[persistent_state(path, data_type, name)]` Share state between nodes over multiple cycles
            - Parameter `#[parameter(data_type, name, path, on_changed)]` Get configuration parameters from the configuration file/via Communication
        - Outputs
            - Main Output `#[main_output(data_type, name)]` Output for dependent nodes, generated in every cycle
            - Additional Output `#[additional_output(path, data_type, name)]` Optional output that can be enabled/requested from e.g. Communication
    - `require_some!` TODO: `required` flag?
        - Extracts data from cycle context and returns none for all main outputs if the input was none
        - `require_some!(...) => match ... { Some(...) => ..., None => return MainOutputs::none() }`
    - 3rd-party macros: `nalgebra::point` or `nalgebra::matrix`
        - Link to 3rd-party documentation
