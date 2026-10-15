use std::{
    future::Future,
    num::NonZeroUsize,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use clap::Parser;
use color_eyre::eyre::{Context as _, OptionExt, Result, ensure, eyre};
use recording::{QueryRecording, Recording};
use ros_z::{
    config::RouterConfigBuilder,
    context::{Context, ContextBuilder},
    node::Node,
};
use ros_z_debug::{
    TargetIdentity,
    query::remote::{RecordingInfo, Request, Response, encode, query_key},
};
use tokio::{sync::mpsc, task::JoinSet};
use zenoh::query::Query;

mod recording;

struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

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
    async fn connect(&self) -> Result<Context> {
        let builder = if let Some(router) = &self.router {
            ContextBuilder::default().with_router_endpoint(router)?
        } else {
            let config = RouterConfigBuilder::new()
                .with_listen_endpoint(&self.listen)
                .build_config()
                .map_err(|error| eyre!(error))?;
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
    let namespace = TargetIdentity::new(args.namespace.clone())?
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
    shutdown: impl Future<Output = ()>,
) -> Result<()> {
    let cancel = Arc::new(AtomicBool::new(false));
    let _cancel_on_drop = CancelOnDrop(Arc::clone(&cancel));
    let recording = QueryRecording::new(recording, name, Arc::clone(&cancel))?
        .with_index_budget(index_budget)?;
    let catalog = recording.info.clone();
    tokio::pin!(shutdown);
    let key = query_key(node.namespace());
    let queryable = node
        .session()
        .declare_queryable(key.clone())
        .complete(true)
        .await
        .map_err(|e| eyre!(e))?;
    let (sender, receiver) = mpsc::channel(16);
    let worker = tokio::task::spawn_blocking(move || run_query_worker(recording, receiver));
    let mut replies = JoinSet::new();
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            Some(_) = replies.join_next(), if !replies.is_empty() => {},
            incoming = queryable.recv_async() => {
                let incoming = incoming.map_err(|e| eyre!(e))?;
                let responses = dispatch_request(&incoming, &catalog, &sender);
                replies.spawn(reply_to_query(incoming, key.clone(), responses));
            }
        }
    }
    replies.abort_all();
    cancel.store(true, Ordering::Relaxed);
    drop(sender);
    worker.await?;
    Ok(())
}

fn run_query_worker(
    mut recording: QueryRecording,
    mut requests: mpsc::Receiver<(Request, mpsc::Sender<Response>)>,
) {
    while let Some((request, reply)) = requests.blocking_recv() {
        if reply.is_closed() {
            continue;
        }
        let Request::Plan {
            instance,
            cursor,
            plan,
        } = request
        else {
            let _ = reply.blocking_send(Response::Catalog(recording.info.clone()));
            continue;
        };
        if instance != recording.info.instance {
            let _ = reply.blocking_send(Response::RecordingChanged);
            continue;
        }
        if let Err(error) = recording.query_plan(&plan, cursor, |result| {
            reply.blocking_send(Response::PlanData(result)).is_ok()
        }) {
            let _ = reply.blocking_send(Response::Error(format!("{error:#}")));
        }
    }
}

fn dispatch_request(
    incoming: &Query,
    catalog: &RecordingInfo,
    worker: &mpsc::Sender<(Request, mpsc::Sender<Response>)>,
) -> Result<mpsc::Receiver<Response>> {
    let payload = incoming
        .payload()
        .filter(|payload| payload.len() <= 64 * 1024)
        .ok_or_eyre("missing or oversized request")?;
    let request: Request = serde_json::from_slice(&payload.to_bytes())?;
    let (reply, responses) = mpsc::channel(2);
    match request {
        // Catalog requests must not wait for long-running index builds.
        Request::Catalog => reply
            .try_send(Response::Catalog(catalog.clone()))
            .map_err(|error| eyre!(error.to_string()))?,
        request => worker
            .try_send((request, reply))
            .map_err(|_| eyre!("query worker busy; retry"))?,
    }
    Ok(responses)
}

async fn reply_to_query(incoming: Query, key: String, responses: Result<mpsc::Receiver<Response>>) {
    let mut responses = match responses {
        Ok(responses) => responses,
        Err(error) => {
            let _ = incoming.reply_err(error.to_string()).await;
            return;
        }
    };
    while let Some(response) = responses.recv().await {
        let bytes = match encode(&response) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = incoming.reply_err(error.to_string()).await;
                return;
            }
        };
        if incoming.reply(&key, bytes).await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_preserves_router_options() {
        let defaults = Arguments::try_parse_from(["mcap-replay", "test.mcap"]).unwrap();
        assert_eq!(defaults.listen, "tcp/0.0.0.0:7447");
        assert_eq!(defaults.index_cache_mib.get(), 256);
        assert!(
            Arguments::try_parse_from(["mcap-replay", "test.mcap", "--index-cache-mib", "0"])
                .is_err()
        );
        assert!(
            Arguments::try_parse_from([
                "mcap-replay",
                "test.mcap",
                "--listen",
                "tcp/127.0.0.1:0",
                "--router",
                "tcp/127.0.0.1:7447"
            ])
            .is_err()
        );
    }
}
