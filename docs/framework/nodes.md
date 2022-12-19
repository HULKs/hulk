# Nodes

Nodes usually contain robotics code and are interchangeable components within cyclers.
Each node is characterized by a `cycle()` function which is called in each cycle.
The function gets node's inputs as parameters to the `cycle()` function and returns node's outputs from it.
In addition, nodes consist of a state which is perserved between cycles.

![node](./node.drawio.png)

Nodes are normal Rust structs where the struct's fields represent the state and a method called `cycle()` in the `impl` of the node represents the `cycle()` function.
This concept allows to write nodes in a very Rusty way.
A node may have multiple inputs of different kinds which can be annotated to the node.
Here is an example node, but for more information see [Macros](./macros.md):

```rust
pub struct SolePressureFilter { // (1)
    left_sole_pressure: LowPassFilter<f32>,
    right_sole_pressure: LowPassFilter<f32>,
}

#[node(control)] // (2)
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

1. Node's state
2. Node declaration with `node` [macro](./macros.md)
3. Configuration parameter of type `f32`
4. Input of type `SensorData`
5. Output of type `SolePressure`
6. Empty `impl` to improve usability of language servers and code linters. If the node declaration would be attached to the `impl` below, when writing incomplete code, the macros would produce errors. This happens a lot if writing node implementation code.
7. Will be called at construction of the node
8. Use declared configuration parameter. Since it is a reference, we need to dereference it with `*`.
9. Will be called every cycle

This node consumes the type `SensorData` as input and produces the output `SolePressure`.
It has two state variables `left_sole_pressure` and `right_sole_pressure`.

This specification of node inputs and outputs leads to a dependency graph which allows to topologically sort nodes s.t. all dependencies are met before executing the node's `cycle()`.
The `build.rs` file automatically sorts nodes based on this graph.
