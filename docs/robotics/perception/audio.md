# Audio and whistle detection

`microphone_recorder` publishes audio samples on `inputs/microphones_samples`.  
`whistle_detection` computes a spectrum and evaluates a configured frequency band against thresholds derived from the background level.
It publishes detections on `detected_whistle` and exposes spectra and detection diagnostics for inspection.
[Microphone recorder](https://github.com/HULKs/hulk/blob/main/crates/nodes/microphone_recorder/src/lib.rs) · [Whistle detector](https://github.com/HULKs/hulk/blob/main/crates/nodes/whistle_detection/src/lib.rs) · [Detection parameters](https://github.com/HULKs/hulk/blob/main/etc/parameters/base/whistle_detection.json5)

`whistle_filter` aggregates detections and publishes `filtered_whistle`.
`game_controller_state_filter` uses the filtered signal along with GameController and ball information when evaluating game-state transitions.
[Whistle filter](https://github.com/HULKs/hulk/blob/main/crates/nodes/whistle_filter/src/lib.rs) · [Game-state filter](https://github.com/HULKs/hulk/blob/main/crates/nodes/game_controller_state_filter/src/lib.rs) · [Team communication](hsl_network.md)
