use clap::Parser;
use color_eyre::Result;
use communication::client::Client;
use tokio::spawn;

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

    let arguments = CommandlineArguments::parse();

    let address = format!("ws://{}:1337", arguments.address);
    let (client, handle) = Client::new(address);
    let task = spawn(client.run());
    handle.connect().await;

    let mut subscription = handle.subscribe_text(arguments.path).await;

    while let Ok(message) = subscription.receiver.recv().await {
        println!("{message:#?}");
    }

    drop(handle);
    task.await?;
    Ok(())
}
