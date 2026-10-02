# Audio

The ROS-Z executable starts separate `microphone_recorder`, `whistle_detection`, and `whistle_filter` nodes.
The microphone recorder reads the configured microphone device and publishes samples on `inputs/microphones_samples` when device initialization succeeds.

At main revision `0711900e0`, both whistle-processing nodes are implemented and wired:

1. `whistle_detection` consumes `Samples` from `inputs/microphones_samples`.
   For each channel it applies a Hann window and FFT, then compares energy in
   the configured detection band against thresholds derived from the spectrum's
   mean and standard deviation. It publishes per-channel flags on
   `detected_whistle`, plus `audio_spectrums` and `detection_infos` for inspection.
2. `whistle_filter` consumes those flags, retains a bounded buffer, and publishes
   `FilteredWhistle` on `filtered_whistle`. The base configuration requires six
   positive flags in a buffer of twenty. These are channel flags, not necessarily
   twenty audio frames. `last_detection` records local wall-clock time on the
   filtered detection's rising edge.
3. `game_controller_state_filter` consumes `filtered_whistle` for game-state
   transitions.

Parameters live in `etc/parameters/base/whistle_detection.json5` and
`whistle_filter.json5`. The detector creates its FFT at startup; changing
`number_audio_samples` requires a restart, otherwise mismatching buffers are
ignored. Source entry points are under `crates/nodes/whistle_detection` and
`crates/nodes/whistle_filter`. Implementation and wiring do not establish
physical detection accuracy for a particular microphone or environment.
See the [robotics overview](../overview.md) for the node and topic framework.
