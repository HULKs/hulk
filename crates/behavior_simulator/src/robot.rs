use std::sync::Arc;

use color_eyre::Result;
use communication::server::Runtime;
use tokio_util::sync::CancellationToken;

use crate::{cycler::BehaviorCycler, interfake::Interfake};

pub struct Robot {
    pub interface: Arc<Interfake>,
    pub cycler: BehaviorCycler<Interfake>,
}

impl Robot {
    pub fn new(index: usize, keep_running: CancellationToken) -> Self {
        let interface: Arc<_> = Interfake::default().into();
        let communication_server = Runtime::<structs::Configuration>::start(
            None::<String>,
            "etc/configuration",
            format!("behavior_simulator{index}"),
            format!("behavior_simulator{index}"),
            2,
            keep_running,
        )
        .unwrap();

        let (control_writer, control_reader) = framework::multiple_buffer_with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
        ]);

        let control_changed = std::sync::Arc::new(tokio::sync::Notify::new());
        let (control_subscribed_outputs_writer, _control_subscribed_outputs_reader) =
            framework::multiple_buffer_with_slots([
                Default::default(),
                Default::default(),
                Default::default(),
            ]);
        let cycler = BehaviorCycler::new(
            control::CyclerInstance::Control,
            interface.clone(),
            control_writer,
            control_changed.clone(),
            communication_server.get_parameters_reader(),
        )
        .unwrap();
        communication_server.register_cycler_instance(
            "Control",
            control_changed,
            control_reader,
            control_subscribed_outputs_writer,
        );

        Self { interface, cycler }
    }

    pub fn cycle(&mut self) -> Result<()> {
        self.cycler.cycle()
    }
}
