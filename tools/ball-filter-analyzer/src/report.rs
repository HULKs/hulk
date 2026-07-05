use std::{fmt::Write, path::Path};

use color_eyre::{Result, eyre::WrapErr};

use crate::{mcap_input::AnalysisInput, records::EventRow};

pub fn write_report(path: &Path, input: &AnalysisInput, events: &[EventRow]) -> Result<()> {
    let mut report = String::new();
    writeln!(report, "# Ball Filter Analysis")?;
    writeln!(report)?;
    writeln!(report, "## Topic Health")?;
    writeln!(report)?;
    writeln!(report, "| Topic | Count | Rate Hz | Max Gap ms | Missing |")?;
    writeln!(report, "| --- | ---: | ---: | ---: | --- |")?;
    for row in &input.topic_health {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} |",
            row.topic,
            row.message_count,
            format_optional_f64(row.average_rate_hz),
            format_optional_f64(row.max_gap_ms),
            row.missing,
        )?;
    }

    writeln!(report)?;
    writeln!(report, "## Rows")?;
    writeln!(report)?;
    writeln!(report, "- Percepts: {}", input.percepts.len())?;
    writeln!(report, "- Tracks: {}", input.tracks.len())?;
    writeln!(report, "- Primary messages: {}", input.primary.len())?;
    writeln!(report, "- Debug messages: {}", input.debug.len())?;
    writeln!(report, "- Decode errors: {}", input.decode_errors.len())?;

    writeln!(report)?;
    writeln!(report, "## Events")?;
    writeln!(report)?;
    if events.is_empty() {
        writeln!(report, "No analyzer events detected.")?;
    } else {
        for event in events.iter().take(100) {
            writeln!(
                report,
                "- {} ns [{}] {}: {}",
                event.time_ns, event.severity, event.event_kind, event.details
            )?;
        }
        if events.len() > 100 {
            writeln!(report, "- ... {} more events", events.len() - 100)?;
        }
    }

    if !input.decode_errors.is_empty() {
        writeln!(report)?;
        writeln!(report, "## Decode Errors")?;
        for error in input.decode_errors.iter().take(50) {
            writeln!(
                report,
                "- {} ns {}: {}",
                error.time_ns, error.topic, error.message
            )?;
        }
    }

    std::fs::write(path, report)
        .wrap_err_with(|| format!("failed to write report {}", path.display()))?;
    Ok(())
}

fn format_optional_f64(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        mcap_input::{AnalysisInput, DecodeErrorRow},
        records::EventRow,
    };

    use super::write_report;

    #[test]
    fn write_report_includes_events_and_decode_errors() {
        let path = std::env::temp_dir().join(format!(
            "ball-filter-analyzer-report-test-{}.md",
            std::process::id()
        ));
        let input = AnalysisInput {
            decode_errors: vec![DecodeErrorRow {
                topic: "<mcap>".to_string(),
                time_ns: -1,
                message: "trailing chunk is truncated".to_string(),
            }],
            ..Default::default()
        };
        let events = vec![EventRow {
            time_ns: -1,
            event_kind: "mcap_stream_warning".to_string(),
            severity: "warning".to_string(),
            track_id: None,
            details: "trailing chunk is truncated".to_string(),
        }];

        write_report(&path, &input, &events).unwrap();

        let report = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_file(path);
        assert!(report.contains("# Ball Filter Analysis"));
        assert!(report.contains("mcap_stream_warning"));
        assert!(report.contains("## Decode Errors"));
        assert!(report.contains("<mcap>"));
    }
}
