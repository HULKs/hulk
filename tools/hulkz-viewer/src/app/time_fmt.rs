use chrono::{DateTime, Local, Utc};

pub(crate) fn format_timestamp(nanos: u64) -> String {
    let secs = nanos / 1_000_000_000;
    let subsec_nanos = (nanos % 1_000_000_000) as u32;
    let Ok(secs_i64) = i64::try_from(secs) else {
        return format!("{nanos} ns");
    };
    let Some(utc) = DateTime::<Utc>::from_timestamp(secs_i64, subsec_nanos) else {
        return format!("{nanos} ns");
    };
    utc.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S%.3f")
        .to_string()
}
