use communication::{Communication, CyclerOutput, HierarchyType, OutputHierarchy};

use serde_json::Value;
use tokio::runtime::{Builder, Runtime};

use crate::value_buffer::ValueBuffer;

pub struct Nao {
    communication: Communication,
    runtime: Runtime,
}

impl Nao {
    pub fn new(address: Option<String>, connect: bool) -> Self {
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
        let _guard = runtime.enter();
        let communication = Communication::new(address, connect);
        Self {
            communication,
            runtime,
        }
    }

    pub fn set_connect(&self, connect: bool) {
        self.runtime
            .block_on(self.communication.set_connect(connect))
    }

    pub fn set_address(&self, address: String) {
        self.runtime
            .block_on(self.communication.set_address(address));
    }

    pub fn subscribe_output(&self, output: CyclerOutput) -> ValueBuffer {
        let _guard = self.runtime.enter();
        ValueBuffer::output(self.communication.clone(), output)
    }

    pub fn subscribe_parameter(&self, path: &str) -> ValueBuffer {
        let _guard = self.runtime.enter();
        ValueBuffer::parameter(self.communication.clone(), path.to_string())
    }

    pub fn get_output_hierarchy(&self) -> Option<OutputHierarchy> {
        self.runtime
            .block_on(self.communication.get_output_hiearchy())
    }

    pub fn get_parameter_hierarchy(&self) -> Option<HierarchyType> {
        self.runtime
            .block_on(self.communication.get_parameter_hiearchy())
    }

    pub fn update_parameter_value(&self, path: &str, value: Value) {
        self.runtime
            .block_on(self.communication.update_parameter_value(path, value));
    }
}
