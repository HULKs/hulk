use std::fs;

use clap::Subcommand;
use color_eyre::Result;

use crate::rsync;

#[derive(Subcommand, Debug)]
pub enum DataCommand {
    /// Prints all available remote datasets
    ListRemote,
    /// Prints all available local datasets and their completion
    ListLocal,
    /// Downloads a dataset to the local machine
    ToLocal {
        /// The dataset name to be downloaded
        #[arg(required = true)]
        dataset_name: String,
    },
    /// Uploads a dataset to the remote machine including annotations
    ToRemote {
        #[arg(required = true)]
        /// The dataset name to be uploaded
        dataset_name: String,
    },
}

pub fn handle(command: &DataCommand) -> Result<()> {
    match command {
        DataCommand::ListLocal => {
            for dataset_path in fs::read_dir("current")?.filter_map(|x| x.ok()) {
                let entries = fs::read_dir(dataset_path.path().join("images"))?;
                let mut annotations = 0;
                let mut images = 0;

                for entry in entries.filter_map(|x| x.ok()) {
                    if "json" == entry.path().extension().unwrap() {
                        annotations += 1;
                    } else if "png" == entry.path().extension().unwrap() {
                        images += 1;
                    }
                }
                println!(
                    "- {} ({annotations}/{images})",
                    dataset_path.file_name().as_os_str().to_str().unwrap(),
                );
            }
        }
        DataCommand::ListRemote => {
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
