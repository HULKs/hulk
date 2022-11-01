use spl_network::{GameControllerReturnMessage, GameControllerStateMessage, SplMessage};
use tokio::runtime::Runtime;

pub struct SplMessageSender;

#[perception_module(cycler_module = spl_network2)]
#[persistent_state(data_type = Runtime, name = foo, path = runtime)]
#[parameter(data_type = bool, name = okay, path = o.k)]
#[input(cycler_instance = SplNetwork, data_type = SplMessage, is_required = true, name = outgoing_spl_message, path = outgoing_spl_message)]
#[main_output(data_type = SplMessage, name = outgoing_spl_message2)]
impl SplMessageSender {}

impl SplMessageSender {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::none())
    }
}
