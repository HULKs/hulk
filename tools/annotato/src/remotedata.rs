use clap::Subcommand;
use color_eyre::Result;

use crate::rsync;

#[derive(Subcommand, Debug)]
pub enum DataCommand {
    List,
    ToLocal {
        #[arg(short, long, required = true)]
        dataset_name: String,
    },
    ToRemote {
        #[arg(short, long, required = true)]
        dataset_name: String,
    },
}

pub fn handle(command: &DataCommand) -> Result<()> {
    match command {
        DataCommand::List => {
            for dataset_name in rsync::rsync_dataset_list()? {
                println!("- {dataset_name}")
            }
        }
        DataCommand::ToLocal { dataset_name } => {
            rsync::rsync_to_local("current", dataset_name)?;
        }
        DataCommand::ToRemote { dataset_name } => {
            rsync::rsync_to_host("current", dataset_name)?;
        }
    }

    Ok(())
}
