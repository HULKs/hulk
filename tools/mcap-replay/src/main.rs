use std::{
    future::Future,
    io::{Error, stderr},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::Parser;
use color_eyre::{
    Report,
    eyre::{Context as _, OptionExt, Result},
};
use recording::{IndexCache, PendingQuery, QueryRecording, Recording};
use ros_z::{
    config::RouterConfigBuilder,
    context::{Context, ContextBuilder},
    node::Node,
    time::Time,
};
use ros_z_debug::{
    TargetIdentity,
    query::{
        plan::QueryPlan,
        remote::{RecordingInfo, Request, Response, encode, query_key},
    },
};
use tokio::{
    signal::ctrl_c,
    spawn,
    sync::mpsc,
    task::{JoinSet, spawn_blocking},
};
use tokio_util::sync::CancellationToken;
use tracing::instrument;
use zenoh::query::Query;

mod directory;
mod recording;

#[derive(Parser)]
#[command(about = "Serve indexed ROS-Z MCAP queries to Twix over Zenoh")]
struct Arguments {
    /// An indexed MCAP file, or a directory to watch recursively.
    file: PathBuf,
    /// Namespace or directory prefix: /replay or a namespace below it.
    #[arg(long, default_value = "/replay")]
    namespace: String,
    /// Connect to an external router instead of starting an embedded router.
    #[arg(long, conflicts_with = "listen")]
    router: Option<String>,
    /// Embedded router listen endpoint. Use port 0 to pick an available port.
    #[arg(long, default_value = "tcp/0.0.0.0:7447")]
    listen: String,
    /// Global memory budget for lazy timestamp indexes, in MiB. Idle indexes are evicted.
    #[arg(long, default_value = "1024")]
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
                .map_err(Error::other)
                .wrap_err("configuring embedded router")?;
            ContextBuilder::default().with_zenoh_config(config)
        };
        builder.build().await.wrap_err("connecting replay server")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt().with_writer(stderr).init();

    let args = Arguments::parse();
    let index_budget = args
        .index_cache_mib
        .get()
        .checked_mul(1024 * 1024)
        .ok_or_eyre("index cache size overflow")?;
    let namespace = TargetIdentity::new(args.namespace.clone())?
        .namespace()
        .to_owned();

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
    let router = router_uris
        .first()
        .ok_or_eyre("embedded router has no reachable listen URI")?;

    for uri in &router_uris {
        println!("Router URI: {uri}");
    }

    let (cache, cleanup) = IndexCache::new(index_budget)?;
    let cache = Arc::new(cache);
    let result = async {
        if args.file.is_dir() {
            return directory::watch(&args.file, &namespace, &context, Arc::clone(&cache), router)
                .await;
        }

        let (node, recording, name) =
            open_recording(&context, &args.file, &namespace, router).await?;
        serve_queries(&node, recording, name, Arc::clone(&cache), async {
            let _ = ctrl_c().await;
        })
        .await
    }
    .await;

    drop(cache);
    cleanup.await.wrap_err("joining index cleanup task")??;
    result
}

async fn open_recording(
    context: &Context,
    path: &Path,
    namespace: &str,
    router: &str,
) -> Result<(Node, Recording, String)> {
    let file = path.to_owned();
    let ns = namespace.to_owned();
    let recording = spawn_blocking(move || Recording::open(&file, &ns))
        .await?
        .wrap_err_with(|| format!("opening recording in {namespace}"))?;
    let node = context
        .create_node("mcap_replay")
        .with_namespace(namespace)
        .build()
        .await
        .wrap_err_with(|| format!("creating node in {namespace}"))?;

    println!("Twix: ./twix {namespace} --router {router}");
    Ok((node, recording, path.display().to_string()))
}

struct PlanRequest {
    instance: String,
    cursor: Time,
    plan: QueryPlan,
    reply: mpsc::Sender<Response>,
}

async fn serve_queries(
    node: &Node,
    recording: Recording,
    name: String,
    cache: Arc<IndexCache>,
    shutdown: impl Future<Output = ()>,
) -> Result<()> {
    let cancel = CancellationToken::new();
    let _cancel_on_drop = cancel.clone().drop_guard();
    let recording = QueryRecording::new(recording, name, cancel.clone(), cache)?;
    let catalog = recording.info.clone();
    tokio::pin!(shutdown);

    let key = query_key(node.namespace());
    let queryable = node
        .session()
        .declare_queryable(key.clone())
        .complete(true)
        .await
        .map_err(Error::other)
        .wrap_err("declaring replay queryable")?;
    let (sender, receiver) = mpsc::channel(16);
    let mut worker = spawn(run_query_worker(
        recording,
        receiver,
        node.namespace().to_owned(),
        cancel.clone(),
    ));
    tracing::info!(path = %catalog.name, namespace = %node.namespace(), "serving recording");
    let mut replies = JoinSet::new();

    let result = loop {
        tokio::select! {
            _ = &mut shutdown => break Ok(()),
            result = &mut worker => return result?,
            Some(_) = replies.join_next(), if !replies.is_empty() => {},
            incoming = queryable.recv_async() => {
                let incoming = match incoming.map_err(Error::other).wrap_err("receiving replay query") {
                    Ok(incoming) => incoming,
                    Err(error) => break Err(error),
                };
                let responses = dispatch_request(&incoming, &catalog, &sender);
                replies.spawn(reply_to_query(incoming, key.clone(), responses));
            }
        }
    };

    replies.abort_all();
    cancel.cancel();
    drop(sender);
    worker.await??;

    result
}

async fn run_query_worker(
    mut recording: QueryRecording,
    mut requests: mpsc::Receiver<PlanRequest>,
    namespace: String,
    stopped: CancellationToken,
) -> Result<()> {
    loop {
        let request = tokio::select! {
            biased;
            _ = stopped.cancelled() => break,
            request = requests.recv() => match request {
                Some(request) => request,
                None => break,
            },
        };
        if request.reply.is_closed() {
            continue;
        }

        recording = process_request(recording, request, &namespace, &stopped).await?;
    }

    spawn_blocking(move || drop(recording)).await?;

    Ok(())
}

#[instrument(skip_all, fields(path = %recording.info.name, namespace = %namespace))]
async fn process_request(
    recording: QueryRecording,
    request: PlanRequest,
    namespace: &str,
    stopped: &CancellationToken,
) -> Result<QueryRecording> {
    if request.instance != recording.info.instance {
        let _ = request.reply.send(Response::RecordingChanged).await;
        return Ok(recording);
    }

    let query = match PendingQuery::new(&recording, request.plan, request.cursor) {
        Ok(query) => query,
        Err(error) => {
            reply_query_error(&recording.info.name, &request.reply, error).await;
            return Ok(recording);
        }
    };
    // Keep these together: tuple drop order releases query pins before recording cleanup.
    let mut work = (query, recording);
    let mut build = None;
    let result = loop {
        let reply = request.reply.clone();
        let (next, step) = spawn_blocking(move || {
            let step = (|| {
                if let Some((source, guard)) = build {
                    work.0.build(&work.1, source, guard)?;
                }
                work.0.advance(&mut work.1, |result| {
                    reply.blocking_send(Response::PlanData(result)).is_ok()
                })
            })();
            (work, step)
        })
        .await?;
        work = next;
        let source = match step {
            Ok(Some(source)) => source,
            finished => break finished.map(|_| ()),
        };

        let guard = tokio::select! {
            biased;
            _ = stopped.cancelled() => break Ok(()),
            _ = request.reply.closed() => break Ok(()),
            guard = work.1.acquire_build() => guard,
        };
        build = Some((source, guard));
    };
    let (query, recording) = work;
    drop(query);
    if let Err(error) = result {
        reply_query_error(&recording.info.name, &request.reply, error).await;
    }
    Ok(recording)
}

async fn reply_query_error(name: &str, reply: &mpsc::Sender<Response>, error: Report) {
    tracing::warn!(path = name, error = %format!("{error:#}"), "query failed");
    let _ = reply.send(Response::Error(format!("{error:#}"))).await;
}

fn dispatch_request(
    incoming: &Query,
    catalog: &RecordingInfo,
    worker: &mpsc::Sender<PlanRequest>,
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
            .wrap_err("sending recording catalog")?,
        Request::Plan {
            instance,
            cursor,
            plan,
        } => worker
            .try_send(PlanRequest {
                instance,
                cursor,
                plan,
                reply,
            })
            .wrap_err("query worker busy; retry")?,
    }
    Ok(responses)
}

async fn reply_to_query(incoming: Query, key: String, responses: Result<mpsc::Receiver<Response>>) {
    let result: Result<()> = async {
        let mut responses = responses?;
        while let Some(response) = responses.recv().await {
            let bytes = encode(&response)?;
            if incoming.reply(&key, bytes).await.is_err() {
                return Ok(());
            }
        }
        Ok(())
    }
    .await;

    if let Err(error) = result {
        let _ = incoming.reply_err(error.to_string()).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_preserves_router_options() {
        let defaults = Arguments::try_parse_from(["mcap-replay", "test.mcap"]).unwrap();
        assert_eq!(defaults.listen, "tcp/0.0.0.0:7447");
        assert_eq!(defaults.index_cache_mib.get(), 1024);
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
