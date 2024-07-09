#![recursion_limit = "256"]
use std::{env::args, fs::File, io::stdout, sync::Arc};

use color_eyre::{
    eyre::{Result, WrapErr},
    install,
};
use ctrlc::set_handler;
use framework::Parameters as FrameworkParameters;
use hardware::IdInterface;
use hardware_interface::{HardwareInterface, Parameters as HardwareParameters};
use hulk::execution::run;
use serde_json::from_reader;
use tokio_util::sync::CancellationToken;

mod camera;
mod force_sensitive_resistor_devices;
mod hardware_interface;
mod intertial_measurement_unit_devices;
mod joint_devices;
mod keyboard_device;
mod sonar_sensor_devices;

pub fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}  {:<18}  {:>5}  {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(stdout())
        .apply()?;
    Ok(())
}

fn main() -> Result<()> {
    setup_logger()?;
    install()?;
    let framework_parameters_path = args()
        .nth(1)
        .unwrap_or("etc/parameters/framework.json".to_string());
    let keep_running = CancellationToken::new();
    set_handler({
        let keep_running = keep_running.clone();
        move || {
            keep_running.cancel();
        }
    })?;

    let file =
        File::open(framework_parameters_path).wrap_err("failed to open framework parameters")?;
    let framework_parameters: FrameworkParameters =
        from_reader(file).wrap_err("failed to parse framework parameters")?;

    let file = File::open(framework_parameters.hardware_parameters)
        .wrap_err("failed to open hardware parameters")?;
    let hardware_parameters: HardwareParameters =
        from_reader(file).wrap_err("failed to parse hardware parameters")?;

    let hardware_interface = HardwareInterface::new(keep_running.clone(), hardware_parameters)
        .wrap_err("failed to create hardware interface")?;

    let ids = hardware_interface.get_ids();

    run(
        Arc::new(hardware_interface),
        framework_parameters.communication_addresses,
        framework_parameters.parameters_directory,
        "logs",
        ids,
        keep_running,
        framework_parameters.recording_intervals,
    )
}
