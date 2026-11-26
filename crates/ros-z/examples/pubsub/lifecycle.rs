//! `cargo run -p ros-z --example pubsub_lifecycle -- /tmp/lifecycle.json`
//! Press Ctrl-C to finish recording, then open the file in Perfetto and follow a `publication_id`.

use std::{env, fs::File, num::NonZeroUsize, time::Duration};

use color_eyre::eyre::{Result, eyre};
use ros_z::{
    context::ContextBuilder,
    qos::{QosDurability, QosProfile},
};
use tracing_chrome::{ChromeLayerBuilder, TraceStyle};
use tracing_subscriber::{filter::Targets, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    let path = env::args_os()
        .nth(1)
        .ok_or_else(|| eyre!("usage: pubsub_lifecycle NEW_OUTPUT.json"))?;
    let (layer, _guard) = ChromeLayerBuilder::new()
        .writer(File::create_new(path)?)
        .include_args(true)
        .include_locations(false)
        .trace_style(TraceStyle::Async)
        .build();
    tracing_subscriber::registry()
        .with(layer.with_filter(Targets::new().with_target("ros_z::pubsub", tracing::Level::TRACE)))
        .try_init()?;
    tokio::select! {
        result = trace_messages() => result?,
        result = tokio::signal::ctrl_c() => result?,
    }
    Ok(())
}

async fn trace_messages() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let source = context.create_node("source").build().await?;
    let sink = context.create_node("sink").build().await?;
    let publisher = source
        .publisher::<String>("lifecycle")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let subscriber = sink.subscriber::<String>("lifecycle").build().await?;
    let mut latest = sink
        .subscriber::<String>("lifecycle")
        .raw()
        .queue_capacity(NonZeroUsize::MIN)
        .build()
        .await?;

    loop {
        // The typed subscriber reads both messages; the depth-one raw queue keeps only the second.
        for message in ["first", "second"] {
            publisher
                .publish(source.clock().now(), &message.to_owned())
                .await?;
            let received = subscriber.recv_with_metadata().await?;
            println!("{}: {}", received.publication_id(), received.message());
        }
        latest.recv().await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
