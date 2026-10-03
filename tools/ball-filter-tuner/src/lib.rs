//! Offline tuning from ordinary ros-z MCAP recordings; no simulator dependency.
mod recording;
mod scoring;
use clap::Parser;
use color_eyre::{Result, eyre::ensure};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use recording::Recording;
use scoring::{Score, evaluate, preserves_baseline_continuity, verify};
use serde::Serialize;
use std::path::PathBuf;
use types::{ball_filter_tuning::SearchProgress, parameters::BallFilterParameters};

#[derive(Clone, Copy, Debug, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceFrame {
    Ground,
    Field,
}

#[derive(Debug, Parser)]
#[command(about = "Tune the production ball filter against labelled ros-z MCAP recordings")]
pub struct Args {
    #[arg(long, required = true, num_args = 1..)]
    pub train: Vec<PathBuf>,
    /// Separate recordings, never used to choose candidates.
    #[arg(long, required = true, num_args = 1..)]
    pub validation: Vec<PathBuf>,
    #[arg(long, default_value = "etc/parameters/base/ball_filter.json5")]
    pub parameters: PathBuf,
    /// Warm-start the search without changing the baseline used to verify recordings.
    #[arg(long)]
    pub initial_parameters: Option<PathBuf>,
    /// Topic prefix in MCAP. Leave empty for the existing recorder's relative topics.
    #[arg(long, default_value = "")]
    pub namespace: String,
    /// TimeWrapper<Vec<Point3<Ground>>>: single labelled ball, empty=absent, missing=unknown.
    #[arg(long, default_value = "simulation/ball_ground_truth")]
    pub reference_topic: String,
    /// Frame of the labelled reference. Field scoring includes recorded ground_to_field.
    #[arg(long, value_enum, default_value = "ground")]
    pub reference_frame: ReferenceFrame,
    #[arg(long, default_value_t = 4096)]
    pub trials: usize,
    #[arg(long, default_value_t = 7)]
    pub seed: u64,
    /// Spatial loss scale: cap=p², missing=1.25p², false track=p². RMSE is uncapped.
    #[arg(long, default_value_t = 2.0)]
    pub penalty_metres: f64,
    #[arg(long)]
    pub output: PathBuf,
}

#[derive(Serialize)]
struct Comparison {
    baseline: Score,
    optimized: Score,
}
#[derive(Serialize)]
struct Report<'a> {
    objective: scoring::Objective,
    training_recordings: &'a [PathBuf],
    validation_recordings: &'a [PathBuf],
    seed: u64,
    trials: usize,
    rejected_candidates: usize,
    rejected_continuity_candidates: usize,
    continuity_policy: &'static str,
    retention_policy: &'static str,
    tuned_parameter_pointers: Vec<&'static str>,
    penalty_metres: f64,
    namespace: &'a str,
    reference_topic: &'a str,
    reference_frame: ReferenceFrame,
    replay_matches_live: bool,
    validation_improved: bool,
    training: Comparison,
    validation: Comparison,
    baseline_parameters: &'a BallFilterParameters,
    initial_parameters: &'a BallFilterParameters,
    optimized_parameters: &'a BallFilterParameters,
}

// Positive covariance entries use logarithmic bounds. Probabilities/thresholds use
// linear bounds. Timeouts, output thresholds, geometry and confidence gates stay
// fixed. Optional per-second decay rates are searched only on enabled baselines.
const BOUNDS: [(f64, f64, bool); 9] = [
    (0.02, 5.0, true),
    (1e-7, 0.03, true),
    (1e-7, 0.1, true),
    (1e-6, 0.1, true),
    (0.02, 20.0, true),
    (0.99, 1.0, false),
    (0.0, 0.3, false), // hidden confidence decay, per second
    (0.0, 4.0, false), // observable unmatched confidence decay, per second
    (0.0, 2.0, false), // unmatched competitor decay under a confirmed leader, per second
];
fn encode(parameters: &BallFilterParameters) -> [f64; 9] {
    let values = [
        parameters.noise.detection_noise.x(),
        parameters.noise.process_noise_resting[0],
        parameters.noise.process_noise_moving[0],
        parameters.noise.process_noise_moving[2],
        parameters.maximum_matching_cost,
        parameters.velocity_decay_factor,
        parameters.hidden_validity_decay_rate.unwrap_or(0.0),
        parameters.visible_missed_validity_decay_rate.unwrap_or(0.0),
        parameters
            .competing_hypothesis_validity_decay_rate
            .unwrap_or(0.0),
    ];
    std::array::from_fn(|i| {
        let (lo, hi, log) = BOUNDS[i];
        let x = f64::from(values[i]).clamp(lo, hi);
        if log {
            (x.ln() - lo.ln()) / (hi.ln() - lo.ln())
        } else {
            (x - lo) / (hi - lo)
        }
    })
}
fn decode(base: &BallFilterParameters, values: [f64; 9]) -> BallFilterParameters {
    let v: [f32; 9] = std::array::from_fn(|i| {
        let (lo, hi, log) = BOUNDS[i];
        if log {
            (lo.ln() + values[i] * (hi.ln() - lo.ln())).exp() as f32
        } else {
            (lo + values[i] * (hi - lo)) as f32
        }
    });
    let mut p = base.clone();
    p.noise.detection_noise.inner.fill(v[0]);
    p.noise.process_noise_resting.fill(v[1]);
    p.noise.process_noise_moving[0] = v[2];
    p.noise.process_noise_moving[1] = v[2];
    p.noise.process_noise_moving[2] = v[3];
    p.noise.process_noise_moving[3] = v[3];
    p.maximum_matching_cost = v[4];
    p.velocity_decay_factor = v[5];
    p.hidden_validity_decay_rate = base.hidden_validity_decay_rate.map(|_| v[6]);
    p.visible_missed_validity_decay_rate = base.visible_missed_validity_decay_rate.map(|_| v[7]);
    p.competing_hypothesis_validity_decay_rate =
        base.competing_hypothesis_validity_decay_rate.map(|_| v[8]);
    p
}

fn warm_start(base: &BallFilterParameters, initial: &BallFilterParameters) -> BallFilterParameters {
    let mut initial = initial.clone();
    // Omitted legacy rates carry no learned value. Keep the new baseline's
    // values, while explicitly learned Some(0) remains a valid warm start.
    initial.hidden_validity_decay_rate = initial
        .hidden_validity_decay_rate
        .or(base.hidden_validity_decay_rate);
    initial.visible_missed_validity_decay_rate = initial
        .visible_missed_validity_decay_rate
        .or(base.visible_missed_validity_decay_rate);
    initial.competing_hypothesis_validity_decay_rate = initial
        .competing_hypothesis_validity_decay_rate
        .or(base.competing_hypothesis_validity_decay_rate);
    decode(base, encode(&initial))
}

fn active_dimensions(base: &BallFilterParameters) -> Vec<usize> {
    (0..6)
        .chain(base.hidden_validity_decay_rate.map(|_| 6))
        .chain(base.visible_missed_validity_decay_rate.map(|_| 7))
        .chain(base.competing_hypothesis_validity_decay_rate.map(|_| 8))
        .collect()
}

fn tuned_parameter_pointers(base: &BallFilterParameters) -> Vec<&'static str> {
    types::ball_filter_tuning::TUNED_PARAMETER_POINTERS
        .iter()
        .copied()
        .filter(|pointer| match *pointer {
            "/hidden_validity_decay_rate" => base.hidden_validity_decay_rate.is_some(),
            "/visible_missed_validity_decay_rate" => {
                base.visible_missed_validity_decay_rate.is_some()
            }
            "/competing_hypothesis_validity_decay_rate" => {
                base.competing_hypothesis_validity_decay_rate.is_some()
            }
            _ => true,
        })
        .collect()
}

pub fn run(args: Args) -> Result<()> {
    run_with_progress(args, |_| Ok(()))
}

pub fn run_with_progress(
    args: Args,
    mut publish: impl FnMut(&SearchProgress) -> Result<()>,
) -> Result<()> {
    for name in ["ball_filter.json5", "report.json"] {
        ensure!(
            !args.output.join(name).exists(),
            "output already exists: {}",
            args.output.join(name).display()
        );
    }
    ensure!(
        args.trials > 0 && args.penalty_metres.is_finite() && args.penalty_metres > 0.0,
        "trials and penalty must be positive"
    );
    for train in &args.train {
        for validation in &args.validation {
            ensure!(
                std::fs::canonicalize(train)? != std::fs::canonicalize(validation)?,
                "training and validation recordings must differ"
            );
        }
    }
    let baseline: BallFilterParameters =
        json5::from_str(&std::fs::read_to_string(&args.parameters)?)?;
    let read = |paths: &[PathBuf]| -> Result<Vec<Recording>> {
        paths
            .iter()
            .map(|p| {
                Recording::read(
                    p,
                    &args.namespace,
                    &args.reference_topic,
                    args.reference_frame,
                )
            })
            .collect()
    };
    let train = read(&args.train)?;
    let validation = read(&args.validation)?;
    for recording in train.iter().chain(&validation) {
        verify(recording, &baseline)?;
    }
    let base_train = evaluate(&train, &baseline, args.penalty_metres)?;
    ensure!(base_train.loss.is_finite(), "training loss is not finite");
    eprintln!(
        "MCAP replay matches live outputs. Baseline training loss: {:.6}",
        base_train.loss
    );
    let baseline_recordings = train
        .iter()
        .map(|recording| {
            evaluate(
                std::slice::from_ref(recording),
                &baseline,
                args.penalty_metres,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let mut rejected_continuity_candidates = 0;
    let mut best = baseline.clone();
    let mut best_values = encode(&best);
    let mut best_loss = base_train.loss;
    let mut best_metrics = (&base_train).into();
    if let Some(path) = &args.initial_parameters {
        let initial = json5::from_str(&std::fs::read_to_string(path)?)?;
        // Old runs may have optimized away track retention. Import only the
        // searched dimensions; every fixed parameter comes from this baseline.
        let initial = warm_start(&baseline, &initial);
        let score = evaluate_candidate(&train, &initial, args.penalty_metres)?;
        if let Some(score) = score.filter(|score| score.loss.is_finite() && score.loss < best_loss)
        {
            if preserves_baseline_continuity(&score, &base_train)
                && preserves_each_recording(
                    &train,
                    &initial,
                    &baseline_recordings,
                    args.penalty_metres,
                )?
            {
                best = initial;
                best_values = encode(&best);
                best_loss = score.loss;
                best_metrics = (&score).into();
            } else {
                eprintln!("Warm start rejected: worsens baseline training continuity");
            }
        }
    }
    let mut rejected_candidates = 0;
    let initial_parameters = best.clone();
    let mut rng = ChaCha8Rng::seed_from_u64(args.seed);
    let mut progress = SearchProgress {
        reference_frame: format!("{:?}", args.reference_frame),
        trials: args.trials as u64,
        baseline: (&base_train).into(),
        best: best_metrics,
        best_parameters: best.clone(),
        ..Default::default()
    };
    publish(&progress)?;
    let active_dimensions = active_dimensions(&baseline);
    for trial in 0..args.trials {
        let mut values = best_values;
        if trial < 2 * active_dimensions.len() {
            values[active_dimensions[trial / 2]] = (trial % 2) as f64;
        } else if trial % 8 == 0 {
            values = std::array::from_fn(|_| rng.random());
        } else {
            let dimensions = if trial % 3 == 0 { 3 } else { 1 };
            let radius = 0.4 * (1.0 - trial as f64 / args.trials as f64) + 0.05;
            for _ in 0..dimensions {
                let i = active_dimensions[rng.random_range(0..active_dimensions.len())];
                values[i] = (values[i] + rng.random_range(-radius..radius)).clamp(0.0, 1.0);
            }
        }
        let candidate = decode(&baseline, values);
        let score = evaluate_candidate(&train, &candidate, args.penalty_metres)?;
        if let Some(score) = score.filter(|score| score.loss.is_finite()) {
            if score.loss < best_loss {
                if !preserves_baseline_continuity(&score, &base_train)
                    || !preserves_each_recording(
                        &train,
                        &candidate,
                        &baseline_recordings,
                        args.penalty_metres,
                    )?
                {
                    rejected_continuity_candidates += 1;
                } else {
                    best_loss = score.loss;
                    best = candidate;
                    best_values = values;
                    progress.best = (&score).into();
                    progress.best_parameters = best.clone();
                    progress.best_trial = (trial + 1) as u64;
                    eprintln!(
                        "Trial {}/{}: training loss {:.6}",
                        trial + 1,
                        args.trials,
                        best_loss
                    );
                }
            }
        } else {
            rejected_candidates += 1;
            eprintln!(
                "Trial {}/{}: rejected numerically unstable candidate",
                trial + 1,
                args.trials
            );
        }
        progress.trial = (trial + 1) as u64;
        if progress.best_trial == progress.trial || trial % 16 == 0 || trial + 1 == args.trials {
            publish(&progress)?;
        }
    }
    let base_validation = evaluate(&validation, &baseline, args.penalty_metres)?;
    let optimized_validation = evaluate(&validation, &best, args.penalty_metres)?;
    ensure!(
        base_validation.loss.is_finite() && optimized_validation.loss.is_finite(),
        "validation loss is not finite"
    );
    progress.validation_baseline = Some((&base_validation).into());
    progress.validation_best = Some((&optimized_validation).into());
    publish(&progress)?;
    let validation_improved = optimized_validation.loss < base_validation.loss;
    let report = Report {
        objective: scoring::OBJECTIVE,
        training_recordings: &args.train,
        validation_recordings: &args.validation,
        seed: args.seed,
        trials: args.trials,
        rejected_candidates,
        rejected_continuity_candidates,
        continuity_policy: "Every training recording and the aggregate must not worsen baseline total missing time, close-range missing time, or longest missing gap (floating-point roundoff only). Held-out data is evaluation only.",
        retention_policy: "Only listed search dimensions may change, including warm starts. Hypothesis timeout, observable-miss timeout, obstacle source-time tolerance, legacy per-frame confidence factors, field-boundary validity decay rate, maximum detection distance and output threshold remain at the capture baseline. Optional hidden/visible-missed/competing-hypothesis confidence rates are searched only when enabled in the baseline (hidden 0..0.3/s; visible-missed 0..4/s; competing 0..2/s). Legacy None rates retain their prior behavior; omitted field decay rate and detection distance retain their disabled legacy defaults.",
        tuned_parameter_pointers: tuned_parameter_pointers(&baseline),
        penalty_metres: args.penalty_metres,
        namespace: &args.namespace,
        reference_topic: &args.reference_topic,
        reference_frame: args.reference_frame,
        replay_matches_live: true,
        validation_improved,
        training: Comparison {
            baseline: base_train,
            optimized: evaluate(&train, &best, args.penalty_metres)?,
        },
        validation: Comparison {
            baseline: base_validation,
            optimized: optimized_validation,
        },
        baseline_parameters: &baseline,
        initial_parameters: &initial_parameters,
        optimized_parameters: &best,
    };
    std::fs::create_dir_all(&args.output)?;
    for (name, value) in [
        ("ball_filter.json5", serde_json::to_value(&best)?),
        ("report.json", serde_json::to_value(&report)?),
    ] {
        use std::io::Write;
        let mut file = std::fs::File::create_new(args.output.join(name))?;
        writeln!(file, "{}", serde_json::to_string_pretty(&value)?)?;
    }
    eprintln!(
        "Validation loss: {:.6} -> {:.6}; improved: {validation_improved}. Results: {}",
        report.validation.baseline.loss,
        report.validation.optimized.loss,
        args.output.display()
    );
    Ok(())
}

// Check each clip so an improvement in an easy scene cannot hide a regression
// in a contested-ball scene. Only evaluate these additional passes for potential
// new bests; ordinary candidates use the aggregate pass alone.
fn preserves_each_recording(
    recordings: &[Recording],
    parameters: &BallFilterParameters,
    baseline_scores: &[Score],
    penalty: f64,
) -> Result<bool> {
    for (recording, baseline) in recordings.iter().zip(baseline_scores) {
        let score = evaluate(std::slice::from_ref(recording), parameters, penalty)?;
        if !preserves_baseline_continuity(&score, baseline) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Each evaluation creates fresh trackers and only borrows the recordings and
/// parameters. A failed numerical update can therefore be discarded without
/// contaminating later candidates. Baseline verification remains strict, and
/// unrelated panics still propagate instead of hiding programming errors.
fn evaluate_candidate(
    recordings: &[Recording],
    parameters: &BallFilterParameters,
    penalty: f64,
) -> Result<Option<Score>> {
    match std::panic::catch_unwind(|| evaluate(recordings, parameters, penalty)) {
        Ok(result) => result.map(Some),
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied());
            match message {
                Some(
                    "Residual covariance matrix is not invertible"
                    | "covariance not invertible"
                    | "distance is nan",
                ) => Ok(None),
                _ => std::panic::resume_unwind(payload),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_and_old_warm_starts_cannot_override_fixed_policy() {
        let baseline: BallFilterParameters = json5::from_str(include_str!(
            "../../../etc/parameters/base/ball_filter.json5"
        ))
        .unwrap();
        let mut old_best = baseline.clone();
        old_best.hypothesis_timeout = std::time::Duration::from_millis(2800);
        old_best.hidden_validity_exponential_decay_factor = 0.976;
        old_best.visible_validity_exponential_decay_factor = 0.8;
        old_best.validity_output_threshold = 2.0;
        old_best.ball_confidence_threshold = 0.1;
        old_best.visible_missed_detection_timeout = std::time::Duration::ZERO;
        old_best.maximum_obstacle_time_difference = std::time::Duration::from_secs(60);
        old_best.field_boundary_validity_decay_rate = 0.0;
        old_best.maximum_detection_distance = 0.0;
        old_best.hidden_validity_decay_rate = Some(0.25);
        old_best.visible_missed_validity_decay_rate = Some(3.0);
        old_best.competing_hypothesis_validity_decay_rate = Some(1.5);
        old_best.noise.detection_noise.inner.fill(1.5);
        let imported = decode(&baseline, encode(&old_best));
        assert!((imported.noise.detection_noise.x() - 1.5).abs() < 1e-6);
        for candidate in [
            imported,
            decode(&baseline, [0.0; 9]),
            decode(&baseline, [1.0; 9]),
        ] {
            let mut actual = serde_json::to_value(candidate).unwrap();
            let mut expected = serde_json::to_value(&baseline).unwrap();
            for pointer in types::ball_filter_tuning::TUNED_PARAMETER_POINTERS {
                *actual.pointer_mut(pointer).unwrap() = serde_json::Value::Null;
                *expected.pointer_mut(pointer).unwrap() = serde_json::Value::Null;
            }
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn optional_decay_search_bounds_preserve_enabled_zero_and_report_actual_dimensions() {
        let mut base: BallFilterParameters = json5::from_str(include_str!(
            "../../../etc/parameters/base/ball_filter.json5"
        ))
        .unwrap();
        let low = decode(&base, [0.0; 9]);
        let high = decode(&base, [1.0; 9]);
        assert_eq!(low.hidden_validity_decay_rate, Some(0.0));
        assert_eq!(low.visible_missed_validity_decay_rate, Some(0.0));
        assert_eq!(low.competing_hypothesis_validity_decay_rate, Some(0.0));
        assert_eq!(high.hidden_validity_decay_rate, Some(0.3));
        assert_eq!(high.visible_missed_validity_decay_rate, Some(4.0));
        assert_eq!(high.competing_hypothesis_validity_decay_rate, Some(2.0));
        assert_eq!(active_dimensions(&base), (0..9).collect::<Vec<_>>());
        assert!(tuned_parameter_pointers(&base).contains(&"/hidden_validity_decay_rate"));
        assert!(tuned_parameter_pointers(&base).contains(&"/visible_missed_validity_decay_rate"));
        assert!(
            tuned_parameter_pointers(&base).contains(&"/competing_hypothesis_validity_decay_rate")
        );

        base.hidden_validity_decay_rate = None;
        assert_eq!(active_dimensions(&base), vec![0, 1, 2, 3, 4, 5, 7, 8]);
        assert!(!tuned_parameter_pointers(&base).contains(&"/hidden_validity_decay_rate"));
        assert!(tuned_parameter_pointers(&base).contains(&"/visible_missed_validity_decay_rate"));
        for value in [0.0, 0.5, 1.0] {
            assert_eq!(decode(&base, [value; 9]).hidden_validity_decay_rate, None);
        }
        base.visible_missed_validity_decay_rate = None;
        assert_eq!(active_dimensions(&base), vec![0, 1, 2, 3, 4, 5, 8]);
        base.competing_hypothesis_validity_decay_rate = None;
        assert_eq!(active_dimensions(&base), (0..6).collect::<Vec<_>>());
        assert!(!tuned_parameter_pointers(&base).contains(&"/visible_missed_validity_decay_rate"));
        let imported = warm_start(&base, &high);
        assert_eq!(imported.hidden_validity_decay_rate, None);
        assert_eq!(imported.visible_missed_validity_decay_rate, None);
        assert_eq!(imported.competing_hypothesis_validity_decay_rate, None);
        assert!(
            !tuned_parameter_pointers(&base).contains(&"/competing_hypothesis_validity_decay_rate")
        );
    }

    #[test]
    fn legacy_warm_start_keeps_new_baseline_rates_but_explicit_zero_is_learned() {
        let base: BallFilterParameters = json5::from_str(include_str!(
            "../../../etc/parameters/base/ball_filter.json5"
        ))
        .unwrap();
        let mut initial = base.clone();
        initial.hidden_validity_decay_rate = None;
        initial.visible_missed_validity_decay_rate = None;
        initial.competing_hypothesis_validity_decay_rate = None;
        let imported = warm_start(&base, &initial);
        assert_eq!(
            imported.competing_hypothesis_validity_decay_rate,
            base.competing_hypothesis_validity_decay_rate
        );
        assert!(
            (imported.hidden_validity_decay_rate.unwrap()
                - base.hidden_validity_decay_rate.unwrap())
            .abs()
                < 1e-7
        );
        assert!(
            (imported.visible_missed_validity_decay_rate.unwrap()
                - base.visible_missed_validity_decay_rate.unwrap())
            .abs()
                < 1e-7
        );
        initial.hidden_validity_decay_rate = Some(0.0);
        initial.visible_missed_validity_decay_rate = Some(0.0);
        initial.competing_hypothesis_validity_decay_rate = Some(0.0);
        let imported = warm_start(&base, &initial);
        assert_eq!(imported.hidden_validity_decay_rate, Some(0.0));
        assert_eq!(imported.visible_missed_validity_decay_rate, Some(0.0));
        assert_eq!(imported.competing_hypothesis_validity_decay_rate, Some(0.0));
    }

    #[test]
    fn legacy_visibility_baseline_stays_disabled_even_with_new_warm_start() {
        let current: BallFilterParameters = json5::from_str(include_str!(
            "../../../etc/parameters/base/ball_filter.json5"
        ))
        .unwrap();
        assert!(!current.visible_missed_detection_timeout.is_zero());
        assert_eq!(current.field_boundary_validity_decay_rate, 2.0);
        assert_eq!(current.maximum_detection_distance, 15.0);
        let mut legacy_json = serde_json::to_value(&current).unwrap();
        let object = legacy_json.as_object_mut().unwrap();
        object.remove("visible_missed_detection_timeout");
        object.remove("maximum_obstacle_time_difference");
        object.remove("field_boundary_validity_decay_rate");
        object.remove("maximum_detection_distance");
        object.remove("hidden_validity_decay_rate");
        object.remove("visible_missed_validity_decay_rate");
        object.remove("competing_hypothesis_validity_decay_rate");
        let legacy: BallFilterParameters = serde_json::from_value(legacy_json).unwrap();
        assert!(legacy.visible_missed_detection_timeout.is_zero());
        assert_eq!(legacy.field_boundary_validity_decay_rate, 0.0);
        assert_eq!(legacy.maximum_detection_distance, 0.0);
        assert_eq!(legacy.hidden_validity_decay_rate, None);
        assert_eq!(legacy.visible_missed_validity_decay_rate, None);
        assert_eq!(legacy.competing_hypothesis_validity_decay_rate, None);
        assert_eq!(
            legacy.maximum_obstacle_time_difference,
            std::time::Duration::from_millis(100)
        );

        // Warm starts import search dimensions, never opt old recordings into
        // visibility behavior absent from their live baseline.
        let candidate = decode(&legacy, encode(&current));
        assert!(candidate.visible_missed_detection_timeout.is_zero());
        assert_eq!(candidate.field_boundary_validity_decay_rate, 0.0);
        assert_eq!(candidate.maximum_detection_distance, 0.0);
        assert_eq!(candidate.hidden_validity_decay_rate, None);
        assert_eq!(candidate.visible_missed_validity_decay_rate, None);
        assert_eq!(candidate.competing_hypothesis_validity_decay_rate, None);
        assert_eq!(
            candidate.maximum_obstacle_time_difference,
            legacy.maximum_obstacle_time_difference
        );
    }
}
