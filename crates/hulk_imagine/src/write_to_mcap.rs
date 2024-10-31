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

                let main_outputs = database_to_values(
                    &database.main_outputs,
                    $cycler_name.to_string(),
                    "main_outputs".to_string(),
                )?;

                let additional_outputs = database_to_values(
                    &database.main_outputs,
                    $cycler_name.to_string(),
                    "additional_outputs".to_string(),
                )?;

                main_outputs
                    .into_iter()
                    .chain(additional_outputs)
                    .map(|(topic, data)| {
                        $mcap_converter.add_to_mcap(topic, &data, index as u32, timing.timestamp)
                    })
                    .collect::<Result<_, _>>()?;
            }
        }
    };
}
