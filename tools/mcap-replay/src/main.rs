use std::{
    num::NonZeroUsize,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

struct CancelOnDrop(Arc<AtomicBool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

use clap::Parser;
use color_eyre::eyre::{Context as _, OptionExt, Result, ensure};
use ros_z::{config::RouterConfigBuilder, context::ContextBuilder, node::Node};
use ros_z_debug::query::remote::{Request, Response, encode, query_key};

mod recording;
#[cfg(test)]
mod tests;
use recording::{QueryRecording, Recording};

#[derive(Parser)]
#[command(about = "Serve indexed ROS-Z MCAP queries to Twix over Zenoh")]
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
    /// Memory budget for lazy timestamp indexes, in MiB. Unused indexes are evicted.
    #[arg(long, default_value_t = NonZeroUsize::new(recording::DEFAULT_INDEX_BYTES / (1024 * 1024)).expect("positive index budget"))]
    index_cache_mib: NonZeroUsize,
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
        builder.build().await.wrap_err("connecting replay server")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Arguments::parse();
    let index_budget = args
        .index_cache_mib
        .get()
        .checked_mul(1024 * 1024)
        .ok_or_eyre("index cache size overflow")?;
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
    serve_queries(&node, recording, name, index_budget, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
}

async fn serve_queries(
    node: &Node,
    recording: Recording,
    name: String,
    index_budget: usize,
    shutdown: impl std::future::Future<Output = ()>,
) -> Result<()> {
    let cancel = Arc::new(AtomicBool::new(false));
    let _cancel_on_drop = CancelOnDrop(Arc::clone(&cancel));
    let mut recording = QueryRecording::new(recording, name, Arc::clone(&cancel))?
        .with_index_budget(index_budget)?;
    let catalog = recording.info.clone();
    tokio::pin!(shutdown);
    let key = query_key(node.namespace());
    let queryable = node
        .session()
        .declare_queryable(key.clone())
        .complete(true)
        .await
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
    let (sender, mut receiver) =
        tokio::sync::mpsc::channel::<(Request, tokio::sync::mpsc::Sender<Response>)>(16);
    let worker = tokio::task::spawn_blocking(move || {
        while let Some((request, reply)) = receiver.blocking_recv() {
            if reply.is_closed() {
                continue;
            }
            let response = match request {
                Request::Plan {
                    instance,
                    cursor,
                    plan,
                } => {
                    if instance != recording.info.instance {
                        Response::RecordingChanged
                    } else {
                        match recording.query_plan(&plan, cursor, |result| {
                            reply.blocking_send(Response::PlanData(result)).is_ok()
                        }) {
                            Ok(()) => continue,
                            Err(error) => Response::Error(format!("{error:#}")),
                        }
                    }
                }
                Request::Catalog => Response::Catalog(recording.info.clone()),
            };
            let _ = reply.blocking_send(response);
        }
    });
    let mut replies = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            Some(_) = replies.join_next(), if !replies.is_empty() => {},
            incoming = queryable.recv_async() => {
                let incoming = incoming.map_err(|e| color_eyre::eyre::eyre!(e))?;
                let request = incoming.payload().filter(|p| p.len() <= 64 * 1024)
                    .ok_or_else(|| color_eyre::eyre::eyre!("missing or oversized request"))
                    .and_then(|p| serde_json::from_slice::<Request>(&p.to_bytes()).map_err(Into::into));
                let (reply, mut result) = tokio::sync::mpsc::channel(2);
                let error = match request {
                    // Metadata discovery must remain responsive during a long index build.
                    Ok(Request::Catalog) => reply.try_send(Response::Catalog(catalog.clone())).err().map(|error| error.to_string()),
                    Ok(request) => sender.try_send((request, reply)).err().map(|_| "query worker busy; retry".to_owned()),
                    Err(error) => Some(error.to_string()),
                };
                let key = key.clone();
                replies.spawn(async move {
                    if let Some(error) = error {
                        let _ = incoming.reply_err(error).await;
                        return;
                    }
                    while let Some(response) = result.recv().await {
                        match encode(&response) {
                            Ok(bytes) => { if incoming.reply(&key, bytes).await.is_err() { break; } }
                            Err(error) => { let _ = incoming.reply_err(error.to_string()).await; break; }
                        }
                    }
                });
            }
        }
    }
    replies.abort_all();
    cancel.store(true, Ordering::Relaxed);
    drop(sender);
    worker.await?;
    Ok(())
}
