//! Offline tuning from ordinary ros-z MCAP recordings; no simulator dependency.
mod recording;
mod scoring;
use clap::Parser;
use color_eyre::{Result, eyre::ensure};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use recording::Recording;
use scoring::{Score, evaluate, verify};
use serde::Serialize;
use std::{path::PathBuf, time::Duration};
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
    /// TimeWrapper<Vec<Point3<Ground>>>: one target, empty=absent, missing=unknown.
    #[arg(long, default_value = "simulation/ball_ground_truth")]
    pub reference_topic: String,
    /// Frame of the labelled reference. Field scoring includes recorded ground_to_field.
    #[arg(long, value_enum, default_value = "ground")]
    pub reference_frame: ReferenceFrame,
    #[arg(long, default_value_t = 4096)]
    pub trials: usize,
    #[arg(long, default_value_t = 7)]
    pub seed: u64,
    /// Metres-equivalent penalty for a missed ball or a track with no ball present.
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
    training_recordings: &'a [PathBuf],
    validation_recordings: &'a [PathBuf],
    seed: u64,
    trials: usize,
    rejected_candidates: usize,
    tuned_parameter_pointers: &'static [&'static str],
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
// linear bounds. Timestamp tolerance, confidence gate and field geometry are fixed.
const BOUNDS: [(f64, f64, bool); 10] = [
    (0.02, 5.0, true),
    (1e-7, 0.03, true),
    (1e-7, 0.1, true),
    (1e-6, 0.1, true),
    (0.02, 20.0, true),
    (0.5, 3.0, false),
    (0.5, 0.999, false),
    (0.95, 1.0, false),
    (0.2, 20.0, true),
    (0.99, 1.0, false),
];
fn encode(parameters: &BallFilterParameters) -> [f64; 10] {
    let values = [
        parameters.noise.detection_noise.x(),
        parameters.noise.process_noise_resting[0],
        parameters.noise.process_noise_moving[0],
        parameters.noise.process_noise_moving[2],
        parameters.maximum_matching_cost,
        parameters.validity_output_threshold,
        parameters.visible_validity_exponential_decay_factor,
        parameters.hidden_validity_exponential_decay_factor,
        parameters.hypothesis_timeout.as_secs_f32(),
        parameters.velocity_decay_factor,
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
fn decode(base: &BallFilterParameters, values: [f64; 10]) -> BallFilterParameters {
    let v: [f32; 10] = std::array::from_fn(|i| {
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
    p.validity_output_threshold = v[5];
    p.visible_validity_exponential_decay_factor = v[6];
    p.hidden_validity_exponential_decay_factor = v[7];
    p.hypothesis_timeout = Duration::from_secs_f32(v[8]);
    p.velocity_decay_factor = v[9];
    p
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
    let mut best = baseline.clone();
    let mut best_values = encode(&best);
    let mut best_loss = base_train.loss;
    let mut best_metrics = (&base_train).into();
    if let Some(path) = &args.initial_parameters {
        let initial = json5::from_str(&std::fs::read_to_string(path)?)?;
        let score = evaluate_candidate(&train, &initial, args.penalty_metres)?;
        if let Some(score) = score.filter(|score| score.loss.is_finite() && score.loss < best_loss)
        {
            best = initial;
            best_values = encode(&best);
            best_loss = score.loss;
            best_metrics = (&score).into();
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
    for trial in 0..args.trials {
        let mut values = best_values;
        if trial < 20 {
            values[trial / 2] = (trial % 2) as f64;
        } else if trial % 8 == 0 {
            values = std::array::from_fn(|_| rng.random());
        } else {
            let dimensions = if trial % 3 == 0 { 3 } else { 1 };
            let radius = 0.4 * (1.0 - trial as f64 / args.trials as f64) + 0.05;
            for _ in 0..dimensions {
                let i = rng.random_range(0..values.len());
                values[i] = (values[i] + rng.random_range(-radius..radius)).clamp(0.0, 1.0);
            }
        }
        let candidate = decode(&baseline, values);
        let score = evaluate_candidate(&train, &candidate, args.penalty_metres)?;
        if let Some(score) = score.filter(|score| score.loss.is_finite()) {
            if score.loss < best_loss {
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
        training_recordings: &args.train,
        validation_recordings: &args.validation,
        seed: args.seed,
        trials: args.trials,
        rejected_candidates,
        tuned_parameter_pointers: types::ball_filter_tuning::TUNED_PARAMETER_POINTERS,
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
