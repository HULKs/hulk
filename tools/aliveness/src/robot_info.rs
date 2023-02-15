use color_eyre::eyre::{eyre, Context, Result};
use configparser::ini::Ini;
use constants::OS_RELEASE_PATH;
use hula_types::Battery;

use zbus::{dbus_proxy, zvariant::Optional, Connection};

// It is unfortunately not possible to deduplicate those values since they are literals
#[dbus_proxy(
    default_service = "org.hulks.hula",
    interface = "org.hulks.hula",
    default_path = "/org/hulks/HuLA"
)]
trait RobotInfo {
    fn body_id(&self) -> zbus::Result<Optional<String>>;
    fn head_id(&self) -> zbus::Result<Optional<String>>;
    fn battery(&self) -> zbus::Result<Optional<Battery>>;
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
        match self.proxy.battery().await {
            Ok(battery) => Option::from(battery),
            Err(_) => None,
        }
    }

    pub async fn body_id(&mut self) -> Option<String> {
        if self.head_id.is_none() {
            if let Ok(head_id) = self.proxy.body_id().await {
                self.head_id = Option::from(head_id)
            }
        }
        self.head_id.clone()
    }

    pub async fn head_id(&mut self) -> Option<String> {
        if self.body_id.is_none() {
            if let Ok(body_id) = self.proxy.body_id().await {
                self.body_id = Option::from(body_id)
            }
        }
        self.body_id.clone()
    }
}

async fn get_hulks_os_version() -> Result<String> {
    let mut os_release = Ini::new();
    os_release.load_async(OS_RELEASE_PATH).await.unwrap();
    os_release
        .get("default", "VERSION_ID")
        .ok_or_else(|| eyre!("no VERSION_ID in {OS_RELEASE_PATH}"))
}
