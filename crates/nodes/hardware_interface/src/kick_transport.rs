use booster::Kick;
use cdr::{CdrLe, Infinite};
use color_eyre::eyre::{Result, WrapErr, eyre};

const KICK_BALL_TOPIC: &str = "rt/kick_ball";

pub struct KickBallPublisher {
    publisher: zenoh::pubsub::Publisher<'static>,
    _session: zenoh::Session,
}

impl KickBallPublisher {
    pub async fn new(session: &zenoh::Session) -> Result<Self> {
        let publisher = session
            .declare_publisher(KICK_BALL_TOPIC)
            .await
            .map_err(|error| {
                eyre!(error).wrap_err(format!("failed to declare `{KICK_BALL_TOPIC}` publisher"))
            })?;

        Ok(Self {
            publisher,
            _session: session.clone(),
        })
    }

    pub async fn publish(&self, kick: &Kick) -> Result<()> {
        let payload = cdr::serialize::<_, _, CdrLe>(kick, Infinite)
            .wrap_err("failed to serialize kick command")?;

        self.publisher.put(payload).await.map_err(|error| {
            eyre!(error).wrap_err(format!("failed to publish `{KICK_BALL_TOPIC}`"))
        })
    }
}
