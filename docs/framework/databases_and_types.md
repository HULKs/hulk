# Databases & Types

Modules may produce non-standard types.
These specific types are defined in the directory `crates/types`.

A database contains all outputs of the nodes within a cycle.
For each cycler one database exists where it stores the outputs.
It is represented by a Rust struct.
If a node requires an input, a reference to the field in the database struct is given to the node.
The fields in the databases may contain Rust's `Option` types of the node types.
Often, an `Option::Some` represents that the field has been generated and can be used.
`Option::None` can be interpreted as a soft-error that the producing node was not able to generate the output in this cycle.
This can happen for example if the camera projection is not valid for a cycle.
But, databases can also contain plain Rust types, for example if the node always produces some output.
More information about the `Option` encoded types is explained in [Error Handling](./error_handling.md) and [Macros](./macros.md).

TODO: Elaborate

TODO: Explain (de-)serialization of types (Example code!)
