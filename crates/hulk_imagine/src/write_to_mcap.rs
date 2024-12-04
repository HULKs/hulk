use std::time::SystemTime;

use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use indicatif::{ProgressIterator, ProgressStyle};
use serde::Serialize;

use buffered_watch::Receiver;

use crate::{
    execution::Replayer, extractor_hardware_interface::ExtractorHardwareInterface,
    mcap_converter::McapConverter,
};

pub fn write_to_mcap<W, D>(
    replayer: &mut Replayer<ExtractorHardwareInterface>,
    cycler_name: &str,
    mcap_converter: &mut McapConverter<W>,
    mut receiver: Receiver<(SystemTime, D)>,
) -> Result<()>
where
    W: std::io::Write + std::io::Seek,
    D: Serialize,
{
    let unknown_indices_error_message =
        format!("could not find recording indices for `{cycler_name}`");

    let timings: Vec<_> = replayer
        .get_recording_indices()
        .get(cycler_name)
        .wrap_err_with(|| unknown_indices_error_message.clone())?
        .iter()
        .collect();

    let progress_style = ProgressStyle::with_template(
        format!("[{{percent:>2}}%] {{wide_bar:.cyan/blue}} {cycler_name}").as_str(),
    )
    .unwrap();
    for (index, timing) in timings
        .iter()
        .enumerate()
        .progress_with_style(progress_style)
    {
        let frame = replayer
            .get_recording_indices_mut()
            .get_mut(cycler_name)
            .wrap_err_with(|| unknown_indices_error_message.clone())?
            .find_latest_frame_up_to(timing.timestamp)
            .wrap_err("failed to find latest frame")?;

        if let Some(frame) = frame {
            replayer
                .replay(cycler_name, frame.timing.timestamp, &frame.data)
                .wrap_err("failed to replay frame")?;

            let (_, database) = &*receiver.borrow_and_mark_as_seen();

            let outputs = crate::mcap_converter::database_to_values(&database)?;

            outputs.into_iter().try_for_each(|(topic, data)| {
                mcap_converter.add_to_mcap(
                    format!("{}.{}", cycler_name, topic),
                    &data,
                    index as u32,
                    timing.timestamp,
                )
            })?;
        }
    }

    Ok(())
}
