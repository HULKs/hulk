use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::collected_commands::CollectedCommands;

pub struct JointCommandOptimizer {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub collected_commands: Input<CollectedCommands, "collected_commands">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub optimized_commands: MainOutput<CollectedCommands>,
}

impl JointCommandOptimizer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let collected_commands = context.collected_commands.clone();

        Ok(MainOutputs {
            optimized_commands: collected_commands.into(),
        })
    }
}
