use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use clap::Parser;
use color_eyre::eyre::{Context as _, Result, bail, ensure};
use ros_z::{
    attachment::EndpointGlobalId,
    config::RouterConfigBuilder,
    context::ContextBuilder,
    node::Node,
    pubsub::RawPublisher,
    qos::{QosDurability, QosHistory, QosProfile},
    time::Time,
};
use ros_z_debug::replay::{ReplayCommand, ReplayStatus, control_key};
use tokio::time::Instant;

mod recording;
#[cfg(test)]
mod tests;
use recording::{Message, Recording};

#[derive(Parser)]
#[command(about = "Serve an indexed ROS-Z MCAP recording to Twix over Zenoh")]
struct Arguments {
    file: PathBuf,
    #[arg(long, default_value = "/replay")]
    namespace: String,
    /// Connect to an external router instead of starting an embedded router.
    #[arg(long, conflicts_with = "listen")]
    router: Option<String>,
    /// Embedded router listen endpoint. Use port 0 to pick an available port.
    #[arg(long, default_value = "tcp/0.0.0.0:7447")]
    listen: String,
}

impl Arguments {
    async fn connect(&self) -> Result<ros_z::context::Context> {
        let builder = if let Some(router) = &self.router {
            ContextBuilder::default().with_router_endpoint(router)?
        } else {
            let config = RouterConfigBuilder::new()
                .with_listen_endpoint(&self.listen)
                .build_config()
                .map_err(|error| color_eyre::eyre::eyre!(error))?;
            ContextBuilder::default().with_zenoh_config(config)
        };
        builder.build().await.wrap_err_with(|| match &self.router {
            Some(router) => format!("connecting to router {router}"),
            None => format!("starting embedded router on {}; use --listen for another address or --router for an existing router", self.listen),
        })
    }
}

async fn publishers(node: &Node, recording: &Recording) -> Result<BTreeMap<u16, RawPublisher>> {
    let mut publishers = BTreeMap::new();
    for (&id, channel) in &recording.channels {
        publishers.insert(
            id,
            node.raw_publisher(
                &channel.topic,
                channel.type_info.clone(),
                channel.schema.clone(),
            )
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::from_depth(1),
                ..Default::default()
            })
            .build()
            .await?,
        );
    }
    Ok(publishers)
}

async fn publish(publishers: &BTreeMap<u16, RawPublisher>, message: Message) -> Result<()> {
    publishers
        .get(&message.header.channel_id)
        .ok_or_else(|| color_eyre::eyre::eyre!("unknown channel"))?
        .publish_with_source_time(
            message.data,
            Time::from_nanos(message.header.publish_time.try_into()?),
        )
        .await?;
    Ok(())
}

async fn seek(
    node: &Node,
    recording: &mut Recording,
    position: u64,
) -> Result<BTreeMap<u16, RawPublisher>> {
    let snapshot = recording.snapshot(position)?;
    let publishers = publishers(node, recording).await?;
    for message in snapshot {
        publish(&publishers, message).await?;
    }
    Ok(publishers)
}

fn update_sources(status: &mut ReplayStatus, publishers: &BTreeMap<u16, RawPublisher>) {
    status.sources = publishers
        .values()
        .map(|publisher| {
            let entity = publisher.entity();
            (
                entity.topic.clone(),
                EndpointGlobalId::from(entity).into_bytes(),
            )
        })
        .collect();
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Arguments::parse();
    let namespace = ros_z_debug::TargetIdentity::new(args.namespace.clone())?
        .namespace()
        .to_owned();
    let recording = Recording::open(&args.file, &namespace)?;
    let context = args.connect().await?;
    let router_uris = if let Some(router) = &args.router {
        vec![router.clone()]
    } else {
        context
            .session()
            .info()
            .locators()
            .await
            .into_iter()
            .map(|locator| locator.to_string())
            .collect::<Vec<_>>()
    };
    ensure!(
        !router_uris.is_empty(),
        "embedded router has no reachable listen URI"
    );
    for uri in &router_uris {
        println!("Router URI: {uri}");
    }
    println!("Twix: ./twix {namespace} --router {}", router_uris[0]);
    let node = context
        .create_node("mcap_replay")
        .with_namespace(&namespace)
        .build()
        .await?;
    let name = args
        .file
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    serve(&node, recording, name, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
}

async fn serve(
    node: &Node,
    mut recording: Recording,
    name: String,
    shutdown: impl std::future::Future<Output = ()>,
) -> Result<()> {
    let namespace = node.namespace();
    let mut status = ReplayStatus {
        instance: uuid::Uuid::new_v4().to_string(),
        recording: name,
        generation: 0,
        start: recording.start,
        end: recording.end,
        position: recording.start,
        playing: false,
        sources: Default::default(),
    };
    let mut publishers = seek(node, &mut recording, status.position).await?;
    update_sources(&mut status, &publishers);
    let key = control_key(namespace);
    let queryable = node
        .session()
        .declare_queryable(key.clone())
        .complete(true)
        .await
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
    // Scrubbing needs only a snapshot. Build the forward stream when Play is pressed.
    let mut reader = None;
    let mut next: Option<Message> = None;
    let mut origin = (Instant::now(), status.position);
    println!(
        "{}: {} topics, {:.3}s; paused. Connect Twix to {namespace}",
        status.recording,
        publishers.len(),
        (status.end - status.start) as f64 / 1e9
    );
    tokio::pin!(shutdown);
    loop {
        let deadline = origin.0
            + Duration::from_nanos(
                next.as_ref()
                    .map_or(status.end, |m| m.header.log_time)
                    .saturating_sub(origin.1),
            );
        tokio::select! {
            _ = &mut shutdown => break,
            query = queryable.recv_async() => {
                let query = query.map_err(|e| color_eyre::eyre::eyre!(e))?;
                if status.playing {
                    let elapsed = origin.0.elapsed().as_nanos().min(u64::MAX as u128) as u64;
                    let limit = next.as_ref().map_or(status.end, |message| message.header.log_time.saturating_sub(1));
                    status.position = status.position.max(origin.1.saturating_add(elapsed).min(limit));
                }
                let command = query.payload().map(|payload| serde_json::from_slice::<ReplayCommand>(&payload.to_bytes())).transpose();
                let result: Result<()> = async {
                    let command = command?;
                    if let Some(command) = command {
                        ensure!(command.instance() == status.instance, "recording changed; refresh playback status");
                        match command {
                            ReplayCommand::Play { .. } => {
                                if status.position < status.end && !status.playing {
                                    if reader.is_none() {
                                        let mut forward = recording.forward(status.position)?;
                                        next = recording.next(&mut forward)?;
                                        reader = Some(forward);
                                    }
                                    origin = (Instant::now(), status.position);
                                    status.playing = true;
                                }
                            }
                            ReplayCommand::Pause { .. } => { status.playing = false; }
                            ReplayCommand::Seek { position, .. } => {
                                if position < status.start || position > status.end { bail!("seek outside recording bounds"); }
                                let replacement = seek(node, &mut recording, position).await?;
                                publishers = replacement;
                                reader = None;
                                next = None;
                                status.position = position;
                                status.playing = false;
                                status.generation += 1;
                                update_sources(&mut status, &publishers);
                                origin = (Instant::now(), position);
                            }
                        }
                    }
                    Ok(())
                }.await;
                let reply = match result {
                    Ok(()) => query.reply(&key, serde_json::to_vec(&status)?).await,
                    Err(error) => query.reply_err(format!("{error:#}")).await,
                };
                if let Err(error) = reply { eprintln!("playback control reply failed: {error}"); }
            }
            _ = tokio::time::sleep_until(deadline), if status.playing => {
                if let Some(message) = next.take() {
                    status.position = message.header.log_time;
                    publish(&publishers, message).await?;
                    next = recording.next(reader.as_mut().expect("playing initializes the forward reader"))?;
                } else {
                    status.position = status.end;
                    status.playing = false;
                }
            }
        }
    }
    Ok(())
}
