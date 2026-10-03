use booster::LowCommand;
use cdr::{CdrLe, Infinite};
use color_eyre::eyre::{Result, WrapErr, eyre};

const JOINT_CONTROL_TOPIC: &str = "rt/joint_ctrl";

pub struct JointControlPublisher {
    publisher: zenoh::pubsub::Publisher<'static>,
    _session: zenoh::Session,
}

impl JointControlPublisher {
    pub async fn new(session: &zenoh::Session) -> Result<Self> {
        let publisher = session
            .declare_publisher(JOINT_CONTROL_TOPIC)
            .await
            .map_err(|error| {
                eyre!(error).wrap_err(format!(
                    "failed to declare `{JOINT_CONTROL_TOPIC}` publisher"
                ))
            })?;

        Ok(Self {
            publisher,
            _session: session.clone(),
        })
    }

    pub async fn publish(&self, low_command: &LowCommand) -> Result<()> {
        let payload = serialize_low_command(low_command)?;
        self.publisher.put(payload).await.map_err(|error| {
            eyre!(error).wrap_err(format!("failed to publish `{JOINT_CONTROL_TOPIC}`"))
        })
    }
}

fn serialize_low_command(low_command: &LowCommand) -> Result<Vec<u8>> {
    cdr::serialize::<_, _, CdrLe>(low_command, Infinite).wrap_err("failed to serialize LowCommand")
}
