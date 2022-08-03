# Error Handling

TODO: Elaborate

- Error Handling
    - 3 ways to handle errors
        - Set a main output to none: Happens when the module is unable to generate this output (e.g. when inputs are not available or there was a temporary error inside of the module)
            - Recoverable, expected to be resolved in the next cycle
        - Return `Err(...)` from `cycle()`
            - Unrecoverable, but framework is allowed to shutdown gracefully, expected that it will not improve in the next cycles/in the future
        - Panic with e.g. `panic!()` or by `unwrap()`ing
            - Unrecoverable, immediate shutdown, kernel will take down the whole process, there is no way to gracefully shutdown
