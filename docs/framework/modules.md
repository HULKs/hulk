# Modules

Modules usually contain robotics code and are interchangeable components within cyclers.
Each module is characterized by a `cycle()` function which is called in each cycle.
The function gets module's inputs as parameters to the `cycle()` function and returns module's outputs from it.
In addition, modules consist of a state which is perserved between cycles.

![module](./module.drawio.png)

Modules are normal Rust structs where the struct's fields represent the state and a method called `cycle()` in the `impl` of the module represents the `cycle()` function.
This concept allows to write modules in a very Rusty way.
A module may have multiple inputs of different kinds which can be annotated to the module.
Here is an example module, but for more information see [Macros](./macros.md):

```rust
pub struct SolePressureFilter { // (1)
    left_sole_pressure: LowPassFilter<f32>,
    right_sole_pressure: LowPassFilter<f32>,
}

#[module(control)] // (2)
#[parameter(path = low_pass_alpha, data_type = f32)] // (3)
#[input(path = sensor_data, data_type = SensorData)] // (4)
#[main_output(data_type = SolePressure)] // (5)
impl SolePressureFilter {} // (6)

impl SolePressureFilter {
    fn new(context: NewContext) -> anyhow::Result<Self> { // (7)
        Ok(Self {
            left_sole_pressure: LowPassFilter::with_alpha(
                0.0,
                *context.low_pass_alpha, // (8)
            ),
            right_sole_pressure: LowPassFilter::with_alpha(
                0.0,
                *context.low_pass_alpha,
            ),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> { // (9)
        let force_sensitive_resistors =
            &require_some!(context.sensor_data).force_sensitive_resistors;

        let left_sole_pressure = force_sensitive_resistors.left.sum();
        self.left_sole_pressure.update(left_sole_pressure);
        let right_sole_pressure = force_sensitive_resistors.right.sum();
        self.right_sole_pressure.update(right_sole_pressure);

        Ok(MainOutputs {
            sole_pressure: Some(SolePressure {
                left: self.left_sole_pressure.state(),
                right: self.right_sole_pressure.state(),
            }),
        })
    }
}
```

1. Module's state
2. Module declaration with `module` [macro](./macros.md)
3. Configuration parameter of type `f32`
4. Input of type `SensorData`
5. Output of type `SolePressure`
6. Empty `impl` to improve usability of language servers and code linters. If the module declaration would be attached to the `impl` below, when writing incomplete code, the macros would produce errors. This happens a lot if writing module implementation code.
7. Will be called at construction of the module
8. Use declared configuration parameter. Since it is a reference, we need to dereference it with `*`.
9. Will be called every cycle

This module consumes the type `SensorData` as input and produces the output `SolePressure`.
It has two state variables `left_sole_pressure` and `right_sole_pressure`.

This specification of module inputs and outputs leads to a dependency graph which allows to topologically sort modules s.t. all dependencies are met before executing the module's `cycle()`.
The `build.rs` file automatically sorts modules based on this graph.
