#[derive(Default)]
pub struct Outputs<MainOutputs, AdditionalOutputs>
where
    MainOutputs: Default,
    AdditionalOutputs: Default,
{
    pub main_outputs: MainOutputs,
    pub additional_outputs: AdditionalOutputs,
}

mod spl_network2 {
    pub struct Cycler<Interface> {
        instance_name: String,
        hardware_interface: std::sync::Arc<Interface>,
        spl_network_writer: framework::Writer<
            crate::Outputs<
                structs::spl_network2::MainOutputs,
                structs::spl_network2::AdditionalOutputs,
            >,
        >,
        spl_network_main_outputs_producer: framework::Producer<structs::spl_network2::MainOutputs>,
        control_main_outputs_reader: framework::Reader<
            crate::Outputs<structs::control::MainOutputs, structs::control::AdditionalOutputs>,
        >,
        configuration_reader: framework::Reader<structs::Configuration>,
        persistent_state: structs::spl_network2::PersistentState,

        counter: spl_network2::message_receiver::Counter,
    }

    impl<Interface> Cycler<Interface>
    where
        Interface: hardware::HardwareInterface + Send + Sync + 'static,
    {
        pub fn new(
            instance_name: &str,
            hardware_interface: std::sync::Arc<Interface>,
            spl_network_writer: framework::Writer<
                crate::Outputs<
                    structs::spl_network2::MainOutputs,
                    structs::spl_network2::AdditionalOutputs,
                >,
            >,
            spl_network_main_outputs_producer: framework::Producer<
                structs::spl_network2::MainOutputs,
            >,
            control_main_outputs_reader: framework::Reader<
                crate::Outputs<structs::control::MainOutputs, structs::control::AdditionalOutputs>,
            >,
            configuration_reader: framework::Reader<structs::Configuration>,
        ) -> anyhow::Result<Self> {
            use anyhow::Context;

            let configuration = configuration_reader.next().clone();
            Ok(Self {
                instance_name: instance_name.to_string(),
                hardware_interface,
                spl_network_writer,
                spl_network_main_outputs_producer,
                control_main_outputs_reader,
                configuration_reader,
                persistent_state: Default::default(),
                counter: spl_network2::message_receiver::Counter::new(
                    spl_network2::message_receiver::NewContext {
                        initial_value: framework::Parameter::from(
                            &configuration.message_receiver.initial_value,
                        ),
                    },
                )
                .context("Failed to call `new` on `Counter` module")?,
            })
        }

        pub fn start(
            mut self,
            keep_running: tokio_util::sync::CancellationToken,
        ) -> anyhow::Result<std::thread::JoinHandle<()>> {
            use anyhow::Context;

            std::thread::Builder::new()
                .name(self.instance_name.clone())
                .spawn(move || {
                    while !keep_running.is_cancelled() {
                        if let Err(error) = self.cycle() {
                            println!("`cycle` returned error: {error:?}");
                            keep_running.cancel();
                        }
                    }
                })
                .context("Failed to spawn thread")
        }

        fn cycle(&mut self) -> anyhow::Result<()> {
            use anyhow::Context;

            {
                let mut database = self.spl_network_writer.next();

                {
                    let configuration = self.configuration_reader.next();

                    let counter_main_outputs = self
                        .counter
                        .cycle(spl_network2::message_receiver::CycleContext {
                            step: framework::Parameter::from(&configuration.message_receiver.step),
                            hardware_interface: framework::HardwareInterface::from(
                                &self.hardware_interface,
                            ),
                        })
                        .context("Failed to call `cycle` on `Counter` module")?;
                    database.main_outputs.value = counter_main_outputs.value.value;
                }

                self.spl_network_main_outputs_producer.announce();

                // TODO: process modules

                self.spl_network_main_outputs_producer
                    .finalize(database.main_outputs.clone());
            }

            Ok(())
        }
    }
}

pub struct Runtime<Interface> {
    spl_network: spl_network2::Cycler<Interface>,
    // TODO: control
}

impl<Interface> Runtime<Interface>
where
    Interface: hardware::HardwareInterface + Send + Sync + 'static,
{
    pub fn new(hardware_interface: std::sync::Arc<Interface>) -> anyhow::Result<Self> {
        use anyhow::Context;

        let initial_configuration = structs::Configuration::default();

        let (configuration_writer, configuration_reader) = framework::n_tuple_buffer_with_slots([
            // TODO: readers + 2 * writers
            initial_configuration.clone(),
            initial_configuration.clone(),
            initial_configuration.clone(),
            initial_configuration.clone(),
        ]);

        let (spl_network_writer, spl_network_reader) = framework::n_tuple_buffer_with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        ]);
        let (control_writer, control_reader) = framework::n_tuple_buffer_with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        ]);

        let (spl_network_main_outputs_producer, spl_network_main_outputs_consumer) =
            framework::future_queue();

        let spl_network = spl_network2::Cycler::new(
            "SplNetwork",
            hardware_interface,
            spl_network_writer,
            spl_network_main_outputs_producer,
            control_reader,
            configuration_reader,
        )
        .context("Failed to create SplNetwork cycler")?;

        Ok(Self { spl_network })
    }

    pub fn run(self, keep_running: tokio_util::sync::CancellationToken) -> anyhow::Result<()> {
        use anyhow::Context;

        let spl_network = self
            .spl_network
            .start(keep_running)
            .context("Failed to start spl_network cycler")?;

        panic_join(spl_network);

        Ok(())
    }
}

fn panic_join(handle: std::thread::JoinHandle<()>) {
    if let Err(error) = handle.join() {
        std::panic::resume_unwind(error)
    }
}
