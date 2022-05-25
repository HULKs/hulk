use std::{
    fs::set_permissions,
    net::{Ipv4Addr, SocketAddrV4},
    os::unix::prelude::PermissionsExt,
    path::Path,
    sync::Arc,
};

use anyhow::Context;
use log::{debug, info};
use thrussh::client;
use thrussh_keys::*;

use crate::{Connection, NaoAddress};

pub fn address_from_nao_number(number: u8, connection: Connection) -> NaoAddress {
    NaoAddress {
        ip: Ipv4Addr::new(
            10,
            match connection {
                Connection::Wireless => 0,
                Connection::Wired => 1,
            },
            24,
            number,
        ),
    }
}

#[derive(Debug)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub exit_status: Option<u32>,
}

struct Client {}

impl client::Handler for Client {
    type Error = anyhow::Error;
    type FutureBool = futures::future::Ready<Result<(Self, bool), Self::Error>>;
    type FutureUnit = futures::future::Ready<Result<(Self, thrussh::client::Session), Self::Error>>;

    fn finished_bool(self, b: bool) -> Self::FutureBool {
        futures::future::ready(Ok((self, b)))
    }

    fn finished(self, sess: thrussh::client::Session) -> Self::FutureUnit {
        futures::future::ready(Ok((self, sess)))
    }

    fn check_server_key(self, _server_public_key: &key::PublicKey) -> Self::FutureBool {
        self.finished_bool(true)
    }
}

async fn create_session<P>(
    nao: Ipv4Addr,
    project_root: P,
    client_handler: Client,
) -> anyhow::Result<client::Handle<Client>>
where
    P: AsRef<Path>,
{
    debug!("naossh connecting to {}", nao);
    let config = Arc::new(thrussh::client::Config::default());
    let ssh_key = project_root.as_ref().join("scripts/ssh_key");
    let key = Arc::new(thrussh_keys::load_secret_key(ssh_key, None)?);
    let mut session =
        thrussh::client::connect(config, SocketAddrV4::new(nao, 22), client_handler).await?;
    if session.authenticate_publickey("nao", key).await? {
        Ok(session)
    } else {
        Err(anyhow::format_err!("Authentication failed"))
    }
}

pub async fn command<P>(nao: Ipv4Addr, command: &str, project_root: P) -> anyhow::Result<Output>
where
    P: AsRef<Path>,
{
    let mut session = create_session(nao, project_root, Client {})
        .await
        .with_context(|| format!("Failed to create ssh session for {}", nao))?;
    let mut channel = session.channel_open_session().await?;
    debug!("exec naossh {} on {}", command, nao);
    channel.exec(true, command).await?;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit_status = None;

    while let Some(msg) = channel.wait().await {
        match msg {
            thrussh::ChannelMsg::ExtendedData { data, ext } => {
                if ext == 1 {
                    stderr.extend_from_slice(&data);
                } else {
                    stdout.extend_from_slice(&data);
                }
            }
            thrussh::ChannelMsg::Data { data } => {
                stdout.extend_from_slice(&data);
            }
            thrussh::ChannelMsg::ExitStatus { exit_status: e } => {
                exit_status = Some(e);
            }
            _ => (),
        }
    }
    Ok(Output {
        stdout: String::from_utf8(stdout)?,
        stderr: String::from_utf8(stderr)?,
        exit_status,
    })
}

pub fn fix_ssh_key_permissions<P>(project_root: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let ssh_key = project_root.as_ref().join("scripts/ssh_key");
    let metadata = ssh_key
        .metadata()
        .context("Failed to get metadata of SSH key")?;
    let mut permissions = metadata.permissions();
    let read_write_for_owner_only = 0o600;
    if permissions.mode() != read_write_for_owner_only {
        permissions.set_mode(read_write_for_owner_only);
        info!("Changed SSH key permissions to {:o}", permissions.mode());
        set_permissions(ssh_key, permissions).context("Failed to set permissions on SSH key")?;
    }

    Ok(())
}
