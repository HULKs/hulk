use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use color_eyre::eyre::{Result, bail};

pub fn format_output_timestamp(time: SystemTime) -> Result<String> {
    let total_seconds = time.duration_since(UNIX_EPOCH)?.as_secs();
    let days = total_seconds / 86_400;
    let seconds_of_day = total_seconds % 86_400;
    let (year, month, day) = civil_from_days(i64::try_from(days)?);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    Ok(format!(
        "{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}Z"
    ))
}

pub fn resolve_output_path(
    output: Option<PathBuf>,
    name_template: Option<&str>,
    now: SystemTime,
) -> Result<PathBuf> {
    match (output, name_template) {
        (Some(output), None) => Ok(output),
        (None, Some(name_template)) => Ok(PathBuf::from(
            name_template.replace("{timestamp}", &format_output_timestamp(now)?),
        )),
        (None, None) => Ok(PathBuf::from(format!(
            "ros-z-record-{}.mcap",
            format_output_timestamp(now)?
        ))),
        (Some(_), Some(_)) => bail!("--output and --name-template are mutually exclusive"),
    }
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };

    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{Duration, UNIX_EPOCH},
    };

    use super::{format_output_timestamp, resolve_output_path};

    #[test]
    fn formats_output_timestamp_for_filenames() {
        let timestamp = format_output_timestamp(UNIX_EPOCH + Duration::from_secs(1_744_329_600))
            .expect("timestamp should format");

        assert_eq!(timestamp, "20250411T000000Z");
    }

    #[test]
    fn expands_output_template() {
        let path = resolve_output_path(
            None,
            Some("capture-{timestamp}.mcap"),
            UNIX_EPOCH + Duration::from_secs(1_744_329_600),
        )
        .expect("template should expand");

        assert_eq!(path, PathBuf::from("capture-20250411T000000Z.mcap"));
    }

    #[test]
    fn default_output_name_uses_product_prefix() {
        let path = resolve_output_path(None, None, UNIX_EPOCH + Duration::from_secs(1_744_329_600))
            .expect("default output path");

        assert_eq!(path, PathBuf::from("ros-z-record-20250411T000000Z.mcap"));
    }
}
