use std::{
    env::current_dir,
    io::{self, ErrorKind},
    net::Ipv4Addr,
    path::PathBuf,
};

use futures::Future;
use log::error;
use tokio::{fs::read_dir, runtime::Runtime, task::JoinHandle};

use crate::{Connection, NaoName, NaoNumber};

pub fn spawn_task_per_element<T, F, C>(
    runtime: &Runtime,
    naos: Vec<T>,
    future_factory: C,
) -> Vec<JoinHandle<F::Output>>
where
    F: Future + Send + 'static,
    C: Fn(T) -> F,
    F::Output: Send + 'static,
{
    naos.into_iter()
        .map(|element| runtime.spawn(future_factory(element)))
        .collect()
}

pub fn block_on_tasks<T>(
    runtime: &Runtime,
    tasks: Vec<JoinHandle<Result<T, anyhow::Error>>>,
) -> anyhow::Result<Vec<T>>
where
{
    let mut outputs = vec![];
    for task in tasks {
        let result = runtime.block_on(task)?;
        match result {
            Ok(output) => outputs.push(output),
            Err(e) => error!("{:#}", e),
        }
    }
    Ok(outputs)
}

pub fn number_to_ip(nao_number: NaoNumber, connection: Connection) -> anyhow::Result<Ipv4Addr> {
    if nao_number == 0 || nao_number > 254 {
        anyhow::bail!("NAO number not in 8bit")
    }
    let subnet = match connection {
        Connection::Wireless => 0,
        Connection::Wired => 1,
    };
    Ok(Ipv4Addr::new(10, subnet, YOUR_TEAM_NUMBER_HERE, nao_number))
}

pub fn number_to_headname(nao_number: NaoNumber) -> NaoName {
    format!("tuhhnao{}", nao_number)
}

pub fn number_from_nao_name(nao_name: &str) -> anyhow::Result<NaoNumber> {
    match regex::Regex::new(r"\D*(\d*)").unwrap().captures(nao_name) {
        Some(captures) => Ok(captures.get(1).unwrap().as_str().parse()?),
        None => Err(anyhow::anyhow!("cannot match headname regex")),
    }
}

pub fn is_wireless_interface(interface_name: &str) -> bool {
    interface_name.contains("wlan0")
}

pub async fn get_project_root() -> io::Result<PathBuf> {
    let path = current_dir()?;
    let ancestors = path.as_path().ancestors();
    for ancestor in ancestors {
        let mut dir = read_dir(ancestor).await?;
        while let Some(child) = dir.next_entry().await? {
            if child.file_name() == ".git" {
                return Ok(child
                    .path()
                    .parent()
                    .expect("No parent found")
                    .to_path_buf());
            }
        }
    }
    Err(io::Error::new(
        ErrorKind::NotFound,
        "Ran out of places to find .git",
    ))
}
