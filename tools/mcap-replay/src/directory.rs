use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fmt::Write as _,
    io::ErrorKind,
    iter::once,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use color_eyre::eyre::{Context as _, OptionExt, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};
use ros_z::context::Context;
use ros_z_debug::TargetIdentity;
use tokio::{
    signal::ctrl_c,
    spawn,
    sync::{oneshot, watch},
    task::{JoinHandle, spawn_blocking},
    time::sleep,
};
use walkdir::{DirEntry, WalkDir};

use crate::{open_recording, recording::IndexCache, serve_queries};

#[derive(PartialEq, Eq)]
struct Stamp {
    length: u64,
    modified: SystemTime,
    #[cfg(unix)]
    inode: u64,
}

impl Stamp {
    fn read(entry: &DirEntry) -> Result<Self> {
        let metadata = entry.metadata()?;
        Ok(Self {
            length: metadata.len(),
            modified: metadata.modified()?,
            #[cfg(unix)]
            inode: metadata.ino(),
        })
    }
}

struct Serving {
    stamp: Stamp,
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

async fn stop_recordings(recordings: impl IntoIterator<Item = (PathBuf, Serving)>) {
    let tasks: Vec<_> = recordings
        .into_iter()
        .map(|(path, entry)| {
            let _ = entry.stop.send(());
            (path, entry.task)
        })
        .collect();

    for (path, task) in tasks {
        if let Err(error) = task.await {
            tracing::error!(path = %path.display(), %error, "recording task failed");
        }
        tracing::info!(path = %path.display(), "withdrew recording");
    }
}

// Escape unusual bytes and the escape marker, preserving ordinary path names.
fn namespace(base: &str, relative: &Path) -> Result<String> {
    let mut result = base.trim_end_matches('/').to_owned();
    let parent = relative
        .parent()
        .ok_or_eyre("recording path has no parent")?;
    let stem = relative
        .file_stem()
        .ok_or_eyre("recording path has no filename")?;

    for part in parent.iter().chain(once(stem)) {
        result.push('/');
        for &byte in part.as_encoded_bytes() {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.') {
                result.push(char::from(byte));
            } else {
                write!(result, "~{byte:02x}")?;
            }
        }
    }
    Ok(TargetIdentity::new(result)?.namespace().to_owned())
}

#[derive(Default)]
struct Scan {
    files: BTreeMap<PathBuf, Stamp>,
    unreadable: Vec<PathBuf>,
}

impl Scan {
    fn is_unreadable(&self, path: &Path) -> bool {
        self.unreadable
            .iter()
            .any(|prefix| path.starts_with(prefix))
    }
}

fn scan(root: &Path) -> Scan {
    let mut scan = Scan::default();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                if error
                    .io_error()
                    .is_some_and(|error| error.kind() == ErrorKind::NotFound)
                {
                    continue;
                }
                let path = error.path().unwrap_or(root);
                tracing::warn!(path = %path.display(), %error, "scanning directory failed");
                scan.unreadable.push(path.to_owned());
                continue;
            }
        };
        if !entry.file_type().is_file() || entry.path().extension() != Some(OsStr::new("mcap")) {
            continue;
        }
        match Stamp::read(&entry) {
            Ok(stamp) => {
                scan.files.insert(entry.into_path(), stamp);
            }
            Err(error) => {
                tracing::warn!(path = %entry.path().display(), error = %format!("{error:#}"), "reading file metadata failed");
                scan.unreadable.push(entry.into_path());
            }
        }
    }
    scan
}

struct Directory<'a> {
    root: PathBuf,
    base: &'a str,
    context: &'a Context,
    cache: Arc<IndexCache>,
    router: &'a str,
    serving: BTreeMap<PathBuf, Serving>,
    failures: BTreeMap<PathBuf, String>,
}

impl Directory<'_> {
    async fn start(&self, path: &Path, stamp: Stamp) -> Result<Serving> {
        let namespace = namespace(self.base, path.strip_prefix(&self.root)?)?;
        let (node, recording, name) =
            open_recording(self.context, path, &namespace, self.router).await?;

        let (stop, stopped) = oneshot::channel();
        let cache = Arc::clone(&self.cache);
        let task = spawn(async move {
            if let Err(error) = serve_queries(&node, recording, name.clone(), cache, async {
                let _ = stopped.await;
            })
            .await
            {
                tracing::error!(path = %name, namespace = %node.namespace(), error = %format!("{error:#}"), "serving recording failed");
            }
        });
        Ok(Serving { stamp, stop, task })
    }

    async fn reconcile(&mut self) -> Result<()> {
        let root = self.root.clone();
        let scan = spawn_blocking(move || scan(&root)).await?;
        let removed: Vec<_> = self
            .serving
            .extract_if(.., |path, entry| match scan.files.get(path) {
                Some(stamp) => stamp != &entry.stamp || entry.task.is_finished(),
                None => !scan.is_unreadable(path),
            })
            .collect();

        stop_recordings(removed).await;
        self.failures
            .retain(|path, _| scan.files.contains_key(path) || scan.is_unreadable(path));

        for (path, stamp) in scan.files {
            if self.serving.contains_key(&path) {
                continue;
            }
            match self.start(&path, stamp).await {
                Ok(entry) => {
                    self.failures.remove(&path);
                    self.serving.insert(path, entry);
                }
                Err(error) => {
                    let error = format!("{error:#}");
                    if self.failures.get(&path) != Some(&error) {
                        tracing::warn!(path = %path.display(), %error, "cannot serve recording; retrying on filesystem changes");
                    }
                    self.failures.insert(path, error);
                }
            }
        }
        Ok(())
    }
}

pub async fn watch(
    root: &Path,
    base: &str,
    context: &Context,
    cache: Arc<IndexCache>,
    router: &str,
) -> Result<()> {
    let root = root
        .canonicalize()
        .wrap_err("resolving watched directory")?;
    let (changed, mut changes) = watch::channel(());
    let mut watcher = recommended_watcher(move |event: notify::Result<Event>| {
        match event {
            Ok(event) if matches!(event.kind, EventKind::Access(_)) => return,
            Err(error) => tracing::error!(%error, "directory watcher failed; rescanning"),
            _ => {}
        }
        changed.send_replace(());
    })?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .wrap_err("watching replay directory")?;
    tracing::info!(path = %root.display(), "watching recordings recursively");

    let mut directory = Directory {
        root,
        base,
        context,
        cache,
        router,
        serving: BTreeMap::new(),
        failures: BTreeMap::new(),
    };
    let run = async {
        loop {
            directory.reconcile().await?;
            changes.changed().await?;
            // Fixed batching window also makes progress under continuous writes.
            sleep(Duration::from_millis(250)).await;
            changes.borrow_and_update();
        }
    };
    let result = tokio::select! {
        result = run => result,
        result = ctrl_c() => result.wrap_err("waiting for shutdown signal"),
    };

    drop(watcher);
    stop_recordings(directory.serving).await;
    result
}
