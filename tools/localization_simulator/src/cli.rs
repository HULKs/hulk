use std::{
    fs::File,
    io::{BufWriter, Write, stdout},
    path::{Path, PathBuf},
};

use clap::{Args, ValueEnum};
use color_eyre::{Result, eyre::Context as _};
use localization_simulator::{
    AssociationMode, Scenario, SimulationConfig, VisualOdometryMode, report::run_analysis,
};

#[derive(Args, Debug)]
pub struct HeadlessArgs {
    /// Built-in trajectory to run.
    #[arg(long, value_enum, conflicts_with = "scenario_file")]
    scenario: Option<ScenarioPreset>,
    /// Load a validated trajectory from JSON5 instead of using a built-in one.
    #[arg(long, value_name = "PATH")]
    scenario_file: Option<PathBuf>,
    /// Load complete validated sensor settings from JSON5.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    /// Override the deterministic random seed.
    #[arg(long)]
    seed: Option<u64>,
    /// Override the field-mark association path.
    #[arg(long, value_enum)]
    association: Option<AssociationPreset>,
    /// Override the visual-odometry input path.
    #[arg(long, value_enum)]
    visual_odometry: Option<VisualOdometryPreset>,
    /// Write the JSON report to this path, or '-' for standard output.
    #[arg(short, long, default_value = "-", value_name = "PATH")]
    output: PathBuf,
    /// Emit compact JSON instead of pretty-printed JSON.
    #[arg(long)]
    compact: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ScenarioPreset {
    Stationary,
    SixDofLoop,
    FieldFigureEightTwice,
    PoseTeleport,
    VoFault,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum AssociationPreset {
    KnownCorrespondences,
    ProductionAssociation,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum VisualOdometryPreset {
    SyntheticDelta,
    ProductionStereo,
}

pub fn run(args: HeadlessArgs) -> Result<()> {
    let scenario = load_scenario(&args)?;
    let mut config = load_config(args.config.as_deref())?;
    if args.config.is_none()
        && matches!(args.scenario, Some(ScenarioPreset::VoFault))
        && config.vo_outlier.is_none()
    {
        config.vo_outlier = Some(SimulationConfig::diagnostic_vo_fault());
    }
    if let Some(seed) = args.seed {
        config.seed = seed;
    }
    if let Some(association) = args.association {
        config.association_mode = association.into();
    }
    if let Some(visual_odometry) = args.visual_odometry {
        config.visual_odometry_mode = visual_odometry.into();
    }
    if args.config.is_none() && config.visual_odometry_mode == VisualOdometryMode::ProductionStereo
    {
        config.vo_translation_sigma_m = 0.0;
        config.vo_rotation_sigma_rad = 0.0;
    }
    config
        .validate()
        .map_err(|message| color_eyre::eyre::eyre!(message))?;

    let report = run_analysis(scenario, config)?;
    write_report(&report, &args.output, args.compact)
}

fn load_scenario(args: &HeadlessArgs) -> Result<Scenario> {
    if let Some(path) = &args.scenario_file {
        return load_json5(path, "scenario");
    }
    Ok(match args.scenario.unwrap_or(ScenarioPreset::SixDofLoop) {
        ScenarioPreset::Stationary => Scenario::stationary(),
        ScenarioPreset::SixDofLoop => Scenario::six_dof_loop(),
        ScenarioPreset::FieldFigureEightTwice => Scenario::field_figure_eight_twice(),
        ScenarioPreset::PoseTeleport => Scenario::pose_teleport(),
        ScenarioPreset::VoFault => Scenario::vo_fault(),
    })
}

fn load_config(path: Option<&Path>) -> Result<SimulationConfig> {
    path.map_or_else(
        || Ok(SimulationConfig::default()),
        |path| load_json5(path, "simulation config"),
    )
}

fn load_json5<T>(path: &Path, description: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let contents = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read {description} from {}", path.display()))?;
    json5::from_str(&contents)
        .wrap_err_with(|| format!("failed to parse {description} from {}", path.display()))
}

fn write_report(
    report: &localization_simulator::report::AnalysisReport,
    output: &Path,
    compact: bool,
) -> Result<()> {
    if output == Path::new("-") {
        let mut writer = BufWriter::new(stdout().lock());
        serialize_report(report, &mut writer, compact)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        return Ok(());
    }

    let file = File::create(output)
        .wrap_err_with(|| format!("failed to create report at {}", output.display()))?;
    let mut writer = BufWriter::new(file);
    serialize_report(report, &mut writer, compact)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn serialize_report<W: Write>(
    report: &localization_simulator::report::AnalysisReport,
    writer: W,
    compact: bool,
) -> Result<()> {
    if compact {
        serde_json::to_writer(writer, report)?;
    } else {
        serde_json::to_writer_pretty(writer, report)?;
    }
    Ok(())
}

impl From<AssociationPreset> for AssociationMode {
    fn from(value: AssociationPreset) -> Self {
        match value {
            AssociationPreset::KnownCorrespondences => Self::KnownCorrespondences,
            AssociationPreset::ProductionAssociation => Self::ProductionAssociation,
        }
    }
}

impl From<VisualOdometryPreset> for VisualOdometryMode {
    fn from(value: VisualOdometryPreset) -> Self {
        match value {
            VisualOdometryPreset::SyntheticDelta => Self::SyntheticDelta,
            VisualOdometryPreset::ProductionStereo => Self::ProductionStereo,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[derive(Parser)]
    struct TestArgs {
        #[command(flatten)]
        headless: HeadlessArgs,
    }

    #[test]
    fn parses_visual_odometry_override() {
        let args =
            TestArgs::try_parse_from(["test", "--visual-odometry", "production-stereo"]).unwrap();

        assert!(matches!(
            args.headless.visual_odometry,
            Some(VisualOdometryPreset::ProductionStereo)
        ));
    }
}
