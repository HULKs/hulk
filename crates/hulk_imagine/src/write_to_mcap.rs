#[macro_export]
macro_rules! write_to_mcap {
    ($receiver: expr, $cycler_name: expr, $mcap_converter: expr, $replayer: expr) => {
        let unknown_indices_error_message =
            format!("could not find recording indices for `$cycler_name`");

        let timings: Vec<_> = $replayer
            .get_recording_indices()
            .get($cycler_name)
            .expect(&unknown_indices_error_message)
            .iter()
            .collect();

        for (index, timing) in timings.iter().enumerate().progress() {
            let frame = $replayer
                .get_recording_indices_mut()
                .get_mut($cycler_name)
                .map(|index| {
                    index
                        .find_latest_frame_up_to(timing.timestamp)
                        .expect("failed to find latest frame")
                })
                .expect(&unknown_indices_error_message);

            if let Some(frame) = frame {
                $replayer
                    .replay($cycler_name, frame.timing.timestamp, &frame.data)
                    .expect("failed to replay frame");

                let (_, database) = &*$receiver.borrow_and_mark_as_seen();

                let outputs = $crate::mcap_converter::database_to_values(
                    &database,
                )?;

                outputs.into_iter().try_for_each(|(topic, data)| {
                    $mcap_converter.add_to_mcap(
                        format!("{}.{}", $cycler_name, topic),
                        &data,
                        index as u32,
                        timing.timestamp,
                    )
                })?;
            }
        }
    };
}
