use color_eyre::eyre::{Result, eyre};
use zenoh::{Config, Session};

pub(super) async fn open_booster_session() -> Result<Session> {
    // Use the peer's background connector without waiting for the bridge at startup.
    let config = Config::from_json5(
        r#"{
            mode: "peer",
            listen: { endpoints: [] },
            connect: { endpoints: ["tcp/127.0.0.1:7448"] },
            scouting: {
                multicast: { enabled: false },
                gossip: { enabled: false },
            },
            open: { return_conditions: { connect_scouted: false } },
            transport: { shared_memory: { enabled: false } },
        }"#,
    )
    .map_err(|error| eyre!("invalid Booster Zenoh config: {error}"))?;

    zenoh::open(config)
        .await
        .map_err(|error| eyre!("failed to open Booster Zenoh session: {error}"))
}
