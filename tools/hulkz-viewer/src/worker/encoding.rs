use std::{num::NonZeroU128, time::Duration};

use hulkz::Timestamp;
use hulkz_stream::StreamRecord;

use crate::model::DisplayedRecord;

pub(super) fn timestamp_from_nanos(nanos: u64) -> Timestamp {
    let id: zenoh::time::TimestampId = NonZeroU128::new(1).expect("non-zero").into();
    Timestamp::new(zenoh::time::NTP64::from(Duration::from_nanos(nanos)), id)
}

pub(super) fn to_nanos(timestamp: &Timestamp) -> u64 {
    timestamp.get_time().as_nanos()
}

pub(super) fn stream_record_to_displayed_record(record: &StreamRecord) -> DisplayedRecord {
    let encoding = record.encoding.to_string();
    let (json_pretty, raw_fallback) = decode_payload(&encoding, &record.payload);

    DisplayedRecord {
        timestamp_nanos: to_nanos(&record.timestamp),
        json_pretty,
        raw_fallback,
    }
}

pub(super) fn decode_payload(encoding: &str, payload: &[u8]) -> (Option<String>, Option<String>) {
    if encoding.to_ascii_lowercase().contains("json") {
        match serde_json::from_slice::<serde_json::Value>(payload) {
            Ok(value) => {
                let pretty =
                    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
                return (Some(pretty), None);
            }
            Err(_) => {
                return (None, Some(text_or_hex_fallback(payload)));
            }
        }
    }

    (None, Some(text_or_hex_fallback(payload)))
}

pub(super) fn text_or_hex_fallback(payload: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(payload) {
        return text.to_string();
    }

    let preview_len = payload.len().min(64);
    let hex_preview = payload[..preview_len]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        "{} bytes (hex preview: {}{})",
        payload.len(),
        hex_preview,
        if payload.len() > preview_len {
            " ..."
        } else {
            ""
        },
    )
}
