use std::{sync::Arc, time::Duration};

use color_eyre::eyre::{Context, Result};
use dbus::nonblock::{Proxy, SyncConnection};
use dbus_tokio::connection::new_system_sync;
use regex::Regex;
use serde::Serialize;
use tokio::{spawn, task::JoinHandle};

#[derive(Debug, Serialize)]
pub enum Service {
    Hal,
    Hula,
    Hulk,
    Lola,
}

#[derive(Copy, Clone, Debug, Serialize)]
pub enum ActiveState {
    Activating,
    Active,
    Deactivating,
    Failed,
    Inactive,
    NotLoaded,
    Reloading,
    Unknown,
}

impl From<String> for ActiveState {
    fn from(string: String) -> Self {
        match string.as_ref() {
            "activating" => ActiveState::Activating,
            "active" => ActiveState::Active,
            "deactivating" => ActiveState::Deactivating,
            "failed" => ActiveState::Failed,
            "inactive" => ActiveState::Inactive,
            "reloading" => ActiveState::Reloading,
            _ => ActiveState::Unknown,
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
pub struct SystemServices {
    pub hal: ActiveState,
    pub hula: ActiveState,
    pub hulk: ActiveState,
    pub lola: ActiveState,
}

impl SystemServices {
    pub async fn query(manager: &ServiceManager) -> Result<Self> {
        Ok(Self {
            hal: manager.get_service_state(Service::Hal).await?,
            hula: manager.get_service_state(Service::Hula).await?,
            hulk: manager.get_service_state(Service::Hulk).await?,
            lola: manager.get_service_state(Service::Lola).await?,
        })
    }
}

pub struct ServiceManager {
    connection: Arc<SyncConnection>,
    handle: JoinHandle<()>,
}

impl ServiceManager {
    pub async fn connect() -> Result<Self> {
        let (resource, connection) =
            new_system_sync().wrap_err("failed to connect to dbus system bus")?;
        let handle = spawn(async {
            let error = resource.await;
            panic!("Lost connection to D-Bus: {error}");
        });
        Ok(Self { connection, handle })
    }

    fn get_systemd_proxy(&self) -> Proxy<Arc<SyncConnection>> {
        Proxy::new(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            Duration::from_millis(500),
            self.connection.clone(),
        )
    }

    async fn get_unit_path(&self, service: Service) -> Result<dbus::Path<'static>, dbus::Error> {
        let service_name = match service {
            Service::Hal => "hal.service",
            Service::Hula => "hula.service",
            Service::Hulk => "hulk.service",
            Service::Lola => "lola.service",
        };
        self.get_systemd_proxy()
            .method_call(
                "org.freedesktop.systemd1.Manager",
                "GetUnit",
                (service_name,),
            )
            .await
            .map(|r: (dbus::Path<'static>,)| r.0)
    }

    pub async fn get_service_state(&self, service: Service) -> Result<ActiveState, dbus::Error> {
        let regex = Regex::new(r"Unit \w+\.service not loaded").unwrap();
        let unit_path = match self.get_unit_path(service).await {
            Ok(unit_path) => unit_path,
            Err(error) if error.message().is_some() && regex.is_match(error.message().unwrap()) => {
                println!("{error}");
                return Ok(ActiveState::NotLoaded);
            }
            Err(error) => {
                return Err(error);
            }
        };
        let service = Proxy::new(
            "org.freedesktop.systemd1",
            unit_path,
            Duration::from_millis(500),
            self.connection.clone(),
        );
        let response = service
            .method_call(
                "org.freedesktop.DBus.Properties",
                "Get",
                ("org.freedesktop.systemd1.Unit", "ActiveState"),
            )
            .await
            .map(|r: (dbus::arg::Variant<String>,)| (r.0).0)?;
        Ok(ActiveState::from(response))
    }
}
