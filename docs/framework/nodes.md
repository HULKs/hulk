# Nodes

Nodes usually contain robotics code and are interchangeable components within cyclers.
Each node is characterized by a `new()` function which is called once at creation and a `cycle()` function which is called in each cycle.
The function gets other node's inputs as parameters to the `cycle()` function and computes an output from that.
In addition, nodes contain a state which is preserved between cycles.

<figure markdown="span">
    ![node](./node.drawio-light.png#only-light)
    ![node](./node.drawio-dark.png#only-dark)
</figure>

Nodes are normal Rust structs where the struct's fields represent the state and a method called `cycle()` in the `impl` of the node represents the `cycle()` function.
This concept allows to write nodes in a very Rusty way.
A node may have multiple inputs of different kinds which can be annotated to the node.
Here is an example node, but for more information see [Macros](./macros.md):

```rust
use std::{collections::VecDeque, time::SystemTime};

use color_eyre::Result;
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use serde::{Deserialize, Serialize};
use types::{cycle_time::CycleTime, filtered_whistle::FilteredWhistle, whistle::Whistle};

#[derive(Deserialize, Serialize)]
pub struct WhistleFilter { // (1)
    detection_buffer: VecDeque<bool>,
    was_detected_last_cycle: bool,
    last_detection: Option<SystemTime>,
}

#[context]
pub struct CreationContext {} // (2)

#[context]
pub struct CycleContext { // (3)
    buffer_length: Parameter<usize, "whistle_filter.buffer_length">, // (4)
    minimum_detections: Parameter<usize, "whistle_filter.minimum_detections">,

    cycle_time: Input<CycleTime, "cycle_time">, // (5)
    detected_whistle: PerceptionInput<Whistle, "Audio", "detected_whistle">, // (6)
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub filtered_whistle: MainOutput<FilteredWhistle>,
}

impl WhistleFilter {
    pub fn new(_context: CreationContext) -> Result<Self> { // (7)
        Ok(Self {
            detection_buffer: Default::default(),
            was_detected_last_cycle: false,
            last_detection: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> { // (9)
        let cycle_start_time = context.cycle_time.start_time;

        for &is_detected in context
            .detected_whistle
            .persistent
            .values()
            .flatten()
            .flat_map(|whistle| &whistle.is_detected)
        {
            self.detection_buffer.push_front(is_detected);
        }
        self.detection_buffer.truncate(*context.buffer_length); // (8)
        let number_of_detections = self
            .detection_buffer
            .iter()
            .filter(|&&was_detected| was_detected)
            .count();
        let is_detected = number_of_detections > *context.minimum_detections;
        let started_this_cycle = is_detected && !self.was_detected_last_cycle;
        if started_this_cycle {
            self.last_detection = Some(cycle_start_time);
        }
        self.was_detected_last_cycle = is_detected;

        Ok(MainOutputs {
            filtered_whistle: FilteredWhistle {
                is_detected,
                last_detection: self.last_detection,
                started_this_cycle,
            }
            .into(),
        })
    }
}
```

1. Node's state
2. Creation context. Its contents are available in the `new(context: CreationContext) -> Result<Self>` function
3. Cycle context. Its contents are available in the `cycle(&mut self, context: CycleContext) -> Result<MainOutputs>` function
4. Parameter from the `default.json`. Can be changed during runtime by e.g. using [twix](../tooling/twix.md).
5. Input from another node of type `CycleTime`.
6. Input from another node, but with persistent and transient data.
7. Will be called at construction of the node
8. Use declared configuration parameter. Since it is a reference, we need to dereference it with `*`.
9. Will be called every cycle

This node consumes the types `CycleTime` and `Whistle` as inputs and produces the output `FilteredWhistle`.
It has three state variables; `detection_buffer`, `was_detected_last_cycle` and `last_detection`.

This specification of node inputs and outputs leads to a dependency graph which allows to topologically sort nodes s.t. all dependencies are met before executing the node's `cycle()`.
The `build.rs` file automatically sorts nodes based on this graph.
