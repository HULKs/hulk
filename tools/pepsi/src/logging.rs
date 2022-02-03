use std::path::Path;

use fern::Dispatch;

pub fn base_logger_config(is_verbose: bool) -> Dispatch {
    let level = if is_verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    Dispatch::new()
        .level_for("thrussh", log::LevelFilter::Warn)
        .level(level)
}

pub fn file_logger_config<P>(log_file: P) -> anyhow::Result<Dispatch>
where
    P: AsRef<Path>,
{
    let dispatch = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(fern::log_file(log_file)?);
    Ok(dispatch)
}

pub fn apply_stdout_logging(is_verbose: bool) -> anyhow::Result<()> {
    let base_config = base_logger_config(is_verbose);
    let stdout_config = Dispatch::new()
        .format(|out, message, record| {
            let colors = fern::colors::ColoredLevelConfig::new();
            out.finish(format_args!(
                "[{}] {}",
                colors.color(record.level()),
                message
            ))
        })
        .chain(std::io::stdout());
    base_config.chain(stdout_config).apply()?;
    Ok(())
}
