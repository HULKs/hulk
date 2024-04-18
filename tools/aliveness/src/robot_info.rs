use color_eyre::eyre::{eyre, Context, Result};
use configparser::ini::Ini;
use constants::OS_RELEASE_PATH;
use hula_types::{Battery, JointsArray};

use tokio::process::Command;
use zbus::{dbus_proxy, zvariant::Optional, Connection};

#[dbus_proxy(
    default_service = "org.hulks.hula",
    interface = "org.hulks.hula",
    default_path = "/org/hulks/HuLA"
)]
trait RobotInfo {
    fn body_id(&self) -> zbus::Result<Optional<String>>;
    fn head_id(&self) -> zbus::Result<Optional<String>>;
    fn battery(&self) -> zbus::Result<Optional<Battery>>;
    fn temperature(&self) -> zbus::Result<Optional<JointsArray>>;
}

pub struct RobotInfo {
    pub hulks_os_version: String,
    pub hostname: String,
    pub body_id: Option<String>,
    pub head_id: Option<String>,
    proxy: RobotInfoProxy<'static>,
}

impl RobotInfo {
    pub async fn initialize(connection: &Connection) -> Result<Self> {
        let hulks_os_version = get_hulks_os_version()
            .await
            .wrap_err("failed to load HULKs-OS version")?;
        let hostname = hostname::get()
            .wrap_err("failed to query hostname")?
            .into_string()
            .map_err(|hostname| eyre!("invalid utf8 in hostname: {hostname:?}"))?;

        let proxy = RobotInfoProxy::new(connection)
            .await
            .wrap_err("failed to connect to dbus proxy")?;

        Ok(Self {
            hulks_os_version,
            hostname,
            body_id: None,
            head_id: None,
            proxy,
        })
    }

    pub async fn battery(&self) -> Option<Battery> {
        self.proxy.battery().await.ok().and_then(Option::from)
    }

    pub async fn temperature(&self) -> Option<JointsArray> {
        self.proxy.temperature().await.ok().and_then(Option::from)
    }

    pub async fn body_id(&mut self) -> Option<String> {
        if self.body_id.is_none() {
            self.body_id = self.proxy.body_id().await.ok().and_then(Option::from)
        }
        self.body_id.clone()
    }

    pub async fn head_id(&mut self) -> Option<String> {
        if self.head_id.is_none() {
            self.head_id = self.proxy.head_id().await.ok().and_then(Option::from)
        }
        self.head_id.clone()
    }
}

async fn get_hulks_os_version() -> Result<String> {
    let mut os_release = Ini::new();
    os_release.load_async(OS_RELEASE_PATH).await.unwrap();
    os_release
        .get("default", "VERSION_ID")
        .ok_or_else(|| eyre!("no VERSION_ID in {OS_RELEASE_PATH}"))
}

pub async fn get_network() -> Result<Option<String>> {
    let output = Command::new("iwctl")
        .arg("station")
        .arg("wlan0")
        .arg("show")
        .output()
        .await
        .wrap_err("failed to execute iwctl command")?;

    Ok(String::from_utf8(output.stdout)
        .wrap_err("failed to decode UTF-8")?
        .lines()
        .find_map(|line| {
            line.split("Connected network")
                .nth(1)
                .map(|string| string.trim().to_owned())
        }))
}
