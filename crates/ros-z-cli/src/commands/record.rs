use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

use color_eyre::eyre::{Result, WrapErr};
use ros_z_recording::{RecordingConfig, RecordingError, RecordingSession, RecordingSummary};

use crate::{app::AppContext, render::text};

pub(crate) fn config_from_args(
    output: Option<PathBuf>,
    topics: Vec<String>,
) -> ros_z_recording::Result<RecordingConfig> {
    let output_path =
        output.unwrap_or_else(|| RecordingConfig::default_output_path(SystemTime::now()));
    let config = RecordingConfig::new(output_path, topics)?;
    if config.output_path().exists() {
        return Err(RecordingError::OutputAlreadyExists {
            path: config.output_path().to_path_buf(),
        });
    }

    Ok(config)
}

pub async fn run(app: &AppContext, config: RecordingConfig) -> Result<()> {
    app.wait_for_graph_settle().await;
    let session = RecordingSession::start(app.node(), app.graph(), config)
        .await
        .wrap_err("failed to start recording")?;
    text::print_record_start(session.output_path(), session.resolved_topics());

    let mut session = session;
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            result.wrap_err("failed to wait for Ctrl-C")?;
        }
        () = session.wait_for_failure() => {}
    }
    finish_stop_result(session.stop().await, &mut io::stdout())
        .wrap_err("failed to stop recording")?;
    Ok(())
}

fn finish_stop_result(
    result: ros_z_recording::Result<RecordingSummary>,
    output: &mut impl io::Write,
) -> Result<()> {
    match result {
        Ok(summary) => {
            text::write_record_summary(output, &summary)?;
            Ok(())
        }
        Err(RecordingError::RecordingStoppedAfterReceiveError { source, summary }) => {
            text::write_record_summary(output, &summary)?;
            Err(RecordingError::RecordingStoppedAfterReceiveError { source, summary }.into())
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::UNIX_EPOCH;

    use ros_z_recording::RecordingError;
    use ros_z_recording::{RecordingSummary, TopicSummary};

    use super::*;

    #[test]
    fn config_from_args_rejects_existing_output_path() {
        let file = tempfile::NamedTempFile::new().expect("temp file");

        let error = config_from_args(Some(file.path().to_path_buf()), vec!["/alpha".to_string()])
            .expect_err("existing output path must fail");

        assert!(matches!(
            error,
            RecordingError::OutputAlreadyExists { path } if path == file.path()
        ));
    }

    #[test]
    fn finish_stop_result_prints_summary_before_returning_receive_error() {
        let summary = RecordingSummary {
            output_path: PathBuf::from("recording.mcap"),
            start_time: UNIX_EPOCH,
            end_time: UNIX_EPOCH,
            topics: vec![TopicSummary {
                topic: "/alpha".to_string(),
                type_name: "test_msgs::Alpha".to_string(),
                schema_hash: "RZHS02_alpha".to_string(),
                messages: 2,
                bytes: 10,
                drops: 1,
            }],
        };
        let error = RecordingError::RecordingStoppedAfterReceiveError {
            source: Box::new(RecordingError::EmptyTopicSelection),
            summary: Box::new(summary),
        };
        let mut output = Vec::new();

        let returned = finish_stop_result(Err(error), &mut output)
            .expect_err("receive error should still be returned");
        let output = String::from_utf8(output).expect("summary should be UTF-8");

        assert!(output.contains("Recording finished"));
        assert!(output.contains("Messages: 2"));
        assert!(output.contains("/alpha  messages=2 bytes=10 drops=1"));
        assert!(returned.to_string().contains("receive error"));
    }
}
