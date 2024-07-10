use clap::Parser;
use color_eyre::Result;

pub fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            let colors = fern::colors::ColoredLevelConfig::new();
            out.finish(format_args!(
                "[{}] {}",
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CommandlineArguments {
    #[clap(short, long, default_value = "localhost")]
    address: String,
    path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logger()?;

    //let arguments = CommandlineArguments::parse();
    //
    //let address = format!("ws://{}:1337", arguments.address);
    //let (connection, handle) = Connection::new(address);
    //let task = spawn(connection.run());
    //
    //drop(handle);
    //task.await?;
    Ok(())
}
