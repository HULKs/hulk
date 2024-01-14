# Directory Structure

The main code repository represents a [monorepo](https://en.wikipedia.org/wiki/Monorepo) containing many parts of the robotic control software and several tools.
The directory structure is organized as follows, only touching parts relevant for the framework:

- `crates/`: Contains several crates relevant to the framework, beside other crates for the robotics domain and tooling
    - `code_generation/`: Once the source code is analyzed, this crate will generate all necessary code to execute all cyclers and nodes
    - `communication/`: The Communication server (for the framework) and client (for debug tooling)
    - `context_attribute/`: Contains the proc-macro `#[context]` used in our nodes to augment and prepare them for the execution in the framework
    - `framework/`: Some basic building blocks (future queue and multiple buffers) and other framework types
    - `parameters/`: Functionality for de/serializing a parameter directory
    - `serialize_hierarchy/`: Traits needed for all types available via Communication
    - `serialize_hierarchy_derive/`: Derive macro for the `SerializeHierarchy` trait
- `etc/`: All additional files necessary when deploying the code to a robot
    - `parameters/`: Parameter files that are deployed to NAOs and are read during startup
- `tools/`: Miscellaneous projects and tools more or less related to the code
    - `pepsi/`: Mainly a tool for deploying and interacting with the NAO
    - `twix/`: Current iteration of a debug tool
