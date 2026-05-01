use std::{collections::BTreeMap, fs, path::PathBuf, time::Duration};

use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use ros_z_record::{
    RecorderOptions, RecordingPlan, RecordingReport, RecordingStartup, StatsSnapshot,
};
use serde_json::json;
use tokio::{task::JoinHandle, time::MissedTickBehavior};
use tokio_util::sync::CancellationToken;

use crate::{
    app::AppContext,
    cli::RecordArgs,
    render::{OutputMode, json as render_json},
};

pub async fn run(
    app: &AppContext,
    output_mode: OutputMode,
    router: &str,
    domain: usize,
    args: &RecordArgs,
) -> Result<()> {
    let options = build_record_options(router, domain, args)?;
    let duration_limit = options.duration_limit;
    let stats_interval = options.stats_interval;

    let plan = RecordingPlan::build(app.node(), options)
        .await
        .map_err(|error| eyre!(error))
        .wrap_err("failed to prepare recorder")?;
    print_startup_summary(output_mode, plan.startup(), duration_limit)?;

    let shutdown = CancellationToken::new();
    let ctrl_c_task = spawn_ctrl_c_watcher(shutdown.clone());
    let duration_task = spawn_duration_watcher(shutdown.clone(), duration_limit);

    let handle = match plan.spawn(shutdown.clone()).await {
        Ok(handle) => handle,
        Err(error) => {
            abort_runtime_tasks(&shutdown, ctrl_c_task, duration_task, None);
            return Err(eyre!(error));
        }
    };

    let stats_task = tokio::spawn(run_stats_loop(
        output_mode,
        stats_interval,
        handle.stats(),
        shutdown.clone(),
    ));
    let report = handle.wait().await.map_err(|error| eyre!(error))?;

    abort_runtime_tasks(&shutdown, ctrl_c_task, duration_task, Some(stats_task));
    print_final_summary(output_mode, &report)?;

    Ok(())
}

fn build_record_options(router: &str, domain: usize, args: &RecordArgs) -> Result<RecorderOptions> {
    let duration_limit = args
        .duration
        .map(|secs| seconds_to_duration("duration", secs))
        .transpose()?;
    let discovery_timeout = seconds_to_duration("discovery-timeout", args.discovery_timeout)?;
    let stats_interval = seconds_to_duration("stats-interval", args.stats_interval)?;
    let output = ros_z_record::resolve_output_path(
        args.output.clone(),
        args.name_template.as_deref(),
        std::time::SystemTime::now(),
    )
    .map_err(|error| eyre!(error))?;

    Ok(RecorderOptions {
        output,
        topics: read_requested_topics(args)?,
        discovery_timeout,
        duration_limit,
        stats_interval,
        session_metadata: session_metadata(router, domain),
    })
}

fn read_requested_topics(args: &RecordArgs) -> Result<Vec<String>> {
    let mut requested_topics = args.topics.clone();
    for topic_file in &args.topic_file {
        requested_topics.extend(read_topic_file(topic_file)?);
    }
    Ok(requested_topics)
}

fn session_metadata(router: &str, domain: usize) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("router".to_string(), router.to_string()),
        ("domain".to_string(), domain.to_string()),
    ])
}

fn seconds_to_duration(name: &str, seconds: f64) -> Result<Duration> {
    if !seconds.is_finite() || seconds <= 0.0 {
        bail!("{name} must be a positive finite number of seconds");
    }

    Ok(Duration::from_secs_f64(seconds))
}

fn read_topic_file(path: &PathBuf) -> Result<Vec<String>> {
    let contents = fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read topic file {}", path.display()))?;
    Ok(parse_topic_file_contents(&contents))
}

fn parse_topic_file_contents(contents: &str) -> Vec<String> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

fn abort_runtime_tasks(
    shutdown: &CancellationToken,
    ctrl_c_task: JoinHandle<()>,
    duration_task: Option<JoinHandle<()>>,
    stats_task: Option<JoinHandle<()>>,
) {
    shutdown.cancel();
    ctrl_c_task.abort();
    if let Some(task) = duration_task {
        task.abort();
    }
    if let Some(task) = stats_task {
        task.abort();
    }
}

fn print_startup_summary(
    output_mode: OutputMode,
    startup: &RecordingStartup,
    duration_limit: Option<Duration>,
) -> Result<()> {
    match output_mode {
        OutputMode::Json => render_json::print_line(&json!({
            "event": "record_startup",
            "duration_limit_s": duration_limit.map(|duration| duration.as_secs_f64()),
            "startup": startup,
        })),
        OutputMode::Text => {
            println!("Recording to {}", startup.output.display());
            println!(
                "Duration: {}",
                duration_limit
                    .map(|duration| format!("{:.3}s", duration.as_secs_f64()))
                    .unwrap_or_else(|| "unbounded".to_string())
            );
            println!("Topics:");
            for topic in &startup.resolved_topics {
                println!(
                    "  {} [{}] schema_hash={} publishers={}",
                    topic.qualified_topic,
                    topic.type_name,
                    topic.schema_hash,
                    topic.publishers.len(),
                );
            }
            Ok(())
        }
    }
}

fn print_stats(output_mode: OutputMode, snapshot: &StatsSnapshot) -> Result<()> {
    match output_mode {
        OutputMode::Json => render_json::print_line(&json!({
            "event": "record_stats",
            "stats": snapshot,
        })),
        OutputMode::Text => {
            println!(
                "Stats: messages={} bytes={}",
                snapshot.total_messages, snapshot.total_bytes
            );
            for topic in &snapshot.topic_stats {
                println!(
                    "  {}: messages={} bytes={}",
                    topic.topic, topic.messages, topic.bytes
                );
            }
            Ok(())
        }
    }
}

fn print_final_summary(output_mode: OutputMode, report: &RecordingReport) -> Result<()> {
    match output_mode {
        OutputMode::Json => render_json::print_line(&json!({
            "event": "record_final",
            "report": report,
        })),
        OutputMode::Text => {
            println!("Finished recording");
            println!("  messages={}", report.total_messages);
            println!("  bytes={}", report.total_bytes);
            if report.silent_topics.is_empty() {
                println!("  silent_topics=none");
            } else {
                println!("  silent_topics={}", report.silent_topics.join(", "));
            }
            Ok(())
        }
    }
}

async fn run_stats_loop(
    output_mode: OutputMode,
    interval_duration: Duration,
    stats_rx: tokio::sync::watch::Receiver<StatsSnapshot>,
    shutdown: CancellationToken,
) {
    let mut interval = tokio::time::interval(interval_duration);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    interval.tick().await;

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = interval.tick() => {
                let snapshot = stats_rx.borrow().clone();
                if let Err(error) = print_stats(output_mode, &snapshot) {
                    eprintln!("failed to print recording stats: {error}");
                }
            }
        }
    }
}

fn spawn_ctrl_c_watcher(shutdown: CancellationToken) -> JoinHandle<()> {
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            shutdown.cancel();
        }
    })
}

fn spawn_duration_watcher(
    shutdown: CancellationToken,
    duration_limit: Option<Duration>,
) -> Option<JoinHandle<()>> {
    duration_limit.map(|duration_limit| {
        tokio::spawn(async move {
            tokio::time::sleep(duration_limit).await;
            shutdown.cancel();
        })
    })
}

#[cfg(test)]
mod tests {
    use super::parse_topic_file_contents;

    #[test]
    fn topic_file_parsing_ignores_comments_and_blank_lines() {
        let parsed = parse_topic_file_contents(
            r#"
            # comment
            /camera

            /imu
            # another comment
            /joint_states
            "#,
        );

        assert_eq!(parsed, vec!["/camera", "/imu", "/joint_states"]);
    }
}
