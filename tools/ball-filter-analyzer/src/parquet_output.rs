use std::{fs::File, path::Path, sync::Arc};

use arrow::{
    array::{
        ArrayRef, BooleanArray, Float32Array, Float64Array, Int32Array, Int64Array, StringArray,
    },
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use color_eyre::{Result, eyre::WrapErr};
use parquet::{arrow::ArrowWriter, basic::Compression, file::properties::WriterProperties};

use crate::{
    mcap_input::AnalysisInput,
    records::{DebugRow, EventRow, PerceptRow, PrimaryRow, TopicHealthRow, TrackRow},
};

pub fn write_outputs(out: &Path, input: &AnalysisInput, events: &[EventRow]) -> Result<()> {
    std::fs::create_dir_all(out)
        .wrap_err_with(|| format!("failed to create output directory {}", out.display()))?;
    write_topic_health(&out.join("topic_health.parquet"), &input.topic_health)?;
    write_percepts(&out.join("percepts.parquet"), &input.percepts)?;
    write_tracks(&out.join("tracks.parquet"), &input.tracks)?;
    write_primary(&out.join("primary.parquet"), &input.primary)?;
    write_debug(&out.join("debug.parquet"), &input.debug)?;
    write_events(&out.join("events.parquet"), events)?;
    Ok(())
}

fn write_batch(path: &Path, schema: Arc<Schema>, columns: Vec<ArrayRef>) -> Result<()> {
    let batch = RecordBatch::try_new(schema.clone(), columns)?;
    let file = File::create(path)
        .wrap_err_with(|| format!("failed to create Parquet file {}", path.display()))?;
    let properties = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();
    let mut writer = ArrowWriter::try_new(file, schema, Some(properties))?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(())
}

fn write_topic_health(path: &Path, rows: &[TopicHealthRow]) -> Result<()> {
    write_batch(
        path,
        topic_health_schema(),
        vec![
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|row| row.topic.as_str()),
            )),
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.message_count),
            )),
            Arc::new(Int64Array::from(
                rows.iter().map(|row| row.first_time_ns).collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(
                rows.iter().map(|row| row.last_time_ns).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.average_rate_hz)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.max_gap_ms).collect::<Vec<_>>(),
            )),
            Arc::new(BooleanArray::from_iter(
                rows.iter().map(|row| Some(row.missing)),
            )),
        ],
    )
}

fn write_percepts(path: &Path, rows: &[PerceptRow]) -> Result<()> {
    write_batch(
        path,
        percepts_schema(),
        vec![
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.time_ns),
            )),
            Arc::new(Int32Array::from_iter_values(
                rows.iter().map(|row| row.percept_index),
            )),
            Arc::new(Float32Array::from_iter_values(rows.iter().map(|row| row.x))),
            Arc::new(Float32Array::from_iter_values(rows.iter().map(|row| row.y))),
            Arc::new(Float32Array::from_iter_values(
                rows.iter().map(|row| row.cov_xx),
            )),
            Arc::new(Float32Array::from_iter_values(
                rows.iter().map(|row| row.cov_xy),
            )),
            Arc::new(Float32Array::from_iter_values(
                rows.iter().map(|row| row.cov_yy),
            )),
            Arc::new(Float32Array::from_iter_values(
                rows.iter().map(|row| row.image_x),
            )),
            Arc::new(Float32Array::from_iter_values(
                rows.iter().map(|row| row.image_y),
            )),
            Arc::new(Float32Array::from_iter_values(
                rows.iter().map(|row| row.image_radius),
            )),
        ],
    )
}

fn write_tracks(path: &Path, rows: &[TrackRow]) -> Result<()> {
    write_batch(
        path,
        tracks_schema(),
        vec![
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.time_ns),
            )),
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.track_id),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|row| row.status.as_str()),
            )),
            Arc::new(Float32Array::from_iter_values(rows.iter().map(|row| row.x))),
            Arc::new(Float32Array::from_iter_values(rows.iter().map(|row| row.y))),
            Arc::new(Float32Array::from_iter_values(
                rows.iter().map(|row| row.vx),
            )),
            Arc::new(Float32Array::from_iter_values(
                rows.iter().map(|row| row.vy),
            )),
            Arc::new(Float32Array::from_iter_values(
                rows.iter().map(|row| row.existence),
            )),
            Arc::new(Float32Array::from_iter_values(
                rows.iter().map(|row| row.cov_xx),
            )),
            Arc::new(Float32Array::from_iter_values(
                rows.iter().map(|row| row.cov_xy),
            )),
            Arc::new(Float32Array::from_iter_values(
                rows.iter().map(|row| row.cov_yy),
            )),
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.last_seen_ns),
            )),
        ],
    )
}

fn write_primary(path: &Path, rows: &[PrimaryRow]) -> Result<()> {
    write_batch(
        path,
        primary_schema(),
        vec![
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.time_ns),
            )),
            Arc::new(Int64Array::from(
                rows.iter().map(|row| row.track_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.status.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float32Array::from(
                rows.iter().map(|row| row.x).collect::<Vec<_>>(),
            )),
            Arc::new(Float32Array::from(
                rows.iter().map(|row| row.y).collect::<Vec<_>>(),
            )),
            Arc::new(Float32Array::from(
                rows.iter().map(|row| row.vx).collect::<Vec<_>>(),
            )),
            Arc::new(Float32Array::from(
                rows.iter().map(|row| row.vy).collect::<Vec<_>>(),
            )),
            Arc::new(Float32Array::from(
                rows.iter().map(|row| row.existence).collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(
                rows.iter().map(|row| row.last_seen_ns).collect::<Vec<_>>(),
            )),
        ],
    )
}

fn write_debug(path: &Path, rows: &[DebugRow]) -> Result<()> {
    write_batch(
        path,
        debug_schema(),
        vec![
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.time_ns),
            )),
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.global_hypothesis_count),
            )),
            Arc::new(Float32Array::from(
                rows.iter()
                    .map(|row| row.best_hypothesis_weight_log)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.track_count),
            )),
            Arc::new(Int64Array::from(
                rows.iter()
                    .map(|row| row.primary_track_id)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.assignment_count),
            )),
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.pruned_hypothesis_count),
            )),
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.pruned_track_count),
            )),
        ],
    )
}

fn write_events(path: &Path, rows: &[EventRow]) -> Result<()> {
    write_batch(
        path,
        events_schema(),
        vec![
            Arc::new(Int64Array::from_iter_values(
                rows.iter().map(|row| row.time_ns),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|row| row.event_kind.as_str()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|row| row.severity.as_str()),
            )),
            Arc::new(Int64Array::from(
                rows.iter().map(|row| row.track_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|row| row.details.as_str()),
            )),
        ],
    )
}

fn topic_health_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("topic", DataType::Utf8, false),
        Field::new("message_count", DataType::Int64, false),
        Field::new("first_time_ns", DataType::Int64, true),
        Field::new("last_time_ns", DataType::Int64, true),
        Field::new("average_rate_hz", DataType::Float64, true),
        Field::new("max_gap_ms", DataType::Float64, true),
        Field::new("missing", DataType::Boolean, false),
    ]))
}

fn percepts_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("time_ns", DataType::Int64, false),
        Field::new("percept_index", DataType::Int32, false),
        Field::new("x", DataType::Float32, false),
        Field::new("y", DataType::Float32, false),
        Field::new("cov_xx", DataType::Float32, false),
        Field::new("cov_xy", DataType::Float32, false),
        Field::new("cov_yy", DataType::Float32, false),
        Field::new("image_x", DataType::Float32, false),
        Field::new("image_y", DataType::Float32, false),
        Field::new("image_radius", DataType::Float32, false),
    ]))
}

fn tracks_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("time_ns", DataType::Int64, false),
        Field::new("track_id", DataType::Int64, false),
        Field::new("status", DataType::Utf8, false),
        Field::new("x", DataType::Float32, false),
        Field::new("y", DataType::Float32, false),
        Field::new("vx", DataType::Float32, false),
        Field::new("vy", DataType::Float32, false),
        Field::new("existence", DataType::Float32, false),
        Field::new("cov_xx", DataType::Float32, false),
        Field::new("cov_xy", DataType::Float32, false),
        Field::new("cov_yy", DataType::Float32, false),
        Field::new("last_seen_ns", DataType::Int64, false),
    ]))
}

fn primary_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("time_ns", DataType::Int64, false),
        Field::new("track_id", DataType::Int64, true),
        Field::new("status", DataType::Utf8, true),
        Field::new("x", DataType::Float32, true),
        Field::new("y", DataType::Float32, true),
        Field::new("vx", DataType::Float32, true),
        Field::new("vy", DataType::Float32, true),
        Field::new("existence", DataType::Float32, true),
        Field::new("last_seen_ns", DataType::Int64, true),
    ]))
}

fn debug_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("time_ns", DataType::Int64, false),
        Field::new("global_hypothesis_count", DataType::Int64, false),
        Field::new("best_hypothesis_weight_log", DataType::Float32, true),
        Field::new("track_count", DataType::Int64, false),
        Field::new("primary_track_id", DataType::Int64, true),
        Field::new("assignment_count", DataType::Int64, false),
        Field::new("pruned_hypothesis_count", DataType::Int64, false),
        Field::new("pruned_track_count", DataType::Int64, false),
    ]))
}

fn events_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("time_ns", DataType::Int64, false),
        Field::new("event_kind", DataType::Utf8, false),
        Field::new("severity", DataType::Utf8, false),
        Field::new("track_id", DataType::Int64, true),
        Field::new("details", DataType::Utf8, false),
    ]))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{mcap_input::AnalysisInput, records::TopicHealthRow};

    use super::write_outputs;

    #[test]
    fn write_outputs_creates_expected_parquet_files() {
        let out =
            std::env::temp_dir().join(format!("ball-filter-analyzer-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(&out).unwrap();

        let input = AnalysisInput {
            topic_health: vec![TopicHealthRow {
                topic: "ball_filter/tracks".to_string(),
                message_count: 1,
                first_time_ns: Some(1),
                last_time_ns: Some(1),
                average_rate_hz: None,
                max_gap_ms: None,
                missing: false,
            }],
            ..Default::default()
        };

        write_outputs(&out, &input, &[]).unwrap();

        assert!(out.join("topic_health.parquet").exists());
        assert!(out.join("events.parquet").exists());
        let _ = fs::remove_dir_all(&out);
    }
}
