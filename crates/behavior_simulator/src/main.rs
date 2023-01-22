use std::io::stdout;

use color_eyre::{eyre::Context, install, Result};
use communication::server::Runtime;
use interfake::Interfake;
use tokio_util::sync::CancellationToken;

mod cycler;
mod interfake;

fn setup_logger(is_verbose: bool) -> Result<(), fern::InitError> {
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
        .level(if is_verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .chain(stdout())
        .apply()?;
    Ok(())
}

fn run(keep_running: CancellationToken) -> Result<()> {
    let interface = Interfake {}.into();
    let communication_server = Runtime::<structs::Configuration>::start(
        Some("[::]:1337"),
        "etc/configuration",
        "behavior_simulator".to_string(),
        "behavior_simulator".to_string(),
        2,
        keep_running.clone(),
    )?;

    let (control_writer, control_reader) = framework::multiple_buffer_with_slots([
        Default::default(),
        Default::default(),
        Default::default(),
    ]);

    let control_changed = std::sync::Arc::new(tokio::sync::Notify::new());
    let (control_subscribed_outputs_writer, control_subscribed_outputs_reader) =
        framework::multiple_buffer_with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
        ]);
    let control_cycler = cycler::Cycler::new(
        control::CyclerInstance::Control,
        interface,
        control_writer,
        control_changed.clone(),
        control_subscribed_outputs_reader,
        communication_server.get_parameters_reader(),
    )?;
    communication_server.register_cycler_instance(
        "Control",
        control_changed,
        control_reader,
        control_subscribed_outputs_writer,
    );

    let control_handle = control_cycler
        .start(keep_running)
        .wrap_err("failed to start cycler `Control`")?;

    let mut encountered_error = false;
    match control_handle.join() {
        Ok(Err(error)) => {
            encountered_error = true;
            println!("{error:?}");
        }
        Err(error) => {
            encountered_error = true;
            println!("{error:?}");
        }
        _ => {}
    }
    match communication_server.join() {
        Ok(Err(error)) => {
            encountered_error = true;
            println!("{error:?}");
        }
        Err(error) => {
            encountered_error = true;
            println!("{error:?}");
        }
        _ => {}
    }

    if encountered_error {
        color_eyre::eyre::bail!("at least one cycler exited with error");
    }
    Ok(())
}

fn main() -> Result<()> {
    setup_logger(true)?;
    install()?;
    let keep_running = CancellationToken::new();
    {
        let keep_running = keep_running.clone();
        ctrlc::set_handler(move || {
            println!("Cancelling...");
            keep_running.cancel();
        })?;
    }
    run(keep_running)
}

fn cancel_on_error<T, E>(keep_running: &CancellationToken, result: Result<T, E>) -> Result<T, E> {
    if result.is_err() {
        keep_running.cancel();
    }
    result
}
