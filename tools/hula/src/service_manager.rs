use std::time::Duration;

use dbus::blocking::{Connection, Proxy};
use dbus::Path;
use log::warn;
use regex::Regex;
use serde::Serialize;

use crate::systemd1::{OrgFreedesktopDBusProperties, OrgFreedesktopSystemd1Manager};

#[derive(Debug, Serialize)]
pub enum Service {
    LOLA,
    HAL,
    HULK,
}

#[derive(Copy, Clone, Debug, Serialize)]
pub enum ActiveState {
    Active,
    Reloading,
    Inactive,
    Failed,
    Activating,
    Deactivating,
    NotLoaded,
    Unknown,
}

impl From<String> for ActiveState {
    fn from(string: String) -> Self {
        match string.as_ref() {
            "active" => ActiveState::Active,
            "reloading" => ActiveState::Reloading,
            "inactive" => ActiveState::Inactive,
            "failed" => ActiveState::Failed,
            "activating" => ActiveState::Activating,
            "deactivating" => ActiveState::Deactivating,
            _ => ActiveState::Unknown,
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
pub struct SystemServices {
    lola_state: ActiveState,
    hal_state: ActiveState,
    hulk_state: ActiveState,
}

impl SystemServices {
    pub fn query(manager: &ServiceManager) -> anyhow::Result<Self> {
        Ok(Self {
            lola_state: manager.get_service_state(Service::LOLA)?,
            hal_state: manager.get_service_state(Service::HAL)?,
            hulk_state: manager.get_service_state(Service::HULK)?,
        })
    }
}

pub struct ServiceManager {
    connection: Connection,
}

impl<'a> ServiceManager {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            connection: Connection::new_system()?,
        })
    }

    fn get_system_bus(&self) -> Proxy<&Connection> {
        self.connection.with_proxy(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            Duration::from_millis(500),
        )
    }

    fn get_unit_path(&self, service: Service) -> Result<Path<'static>, dbus::Error> {
        let service_name = match service {
            Service::LOLA => "lola.service",
            Service::HAL => "hal.service",
            Service::HULK => "hulk.service",
        };
        self.get_system_bus().get_unit(service_name)
    }

    pub fn get_service_state(&self, service: Service) -> Result<ActiveState, dbus::Error> {
        let regex = Regex::new(r"Unit \w+\.service not loaded").unwrap();
        let unit_path = match self.get_unit_path(service) {
            Ok(unit_path) => unit_path,
            Err(error) if error.message().is_some() && regex.is_match(error.message().unwrap()) => {
                warn!("{:?}", error);
                return Ok(ActiveState::NotLoaded);
            }
            Err(error) => {
                return Err(error);
            }
        };
        let service = self.connection.with_proxy(
            "org.freedesktop.systemd1",
            unit_path,
            Duration::from_millis(500),
        );
        let response: String = service.get("org.freedesktop.systemd1.Unit", "ActiveState")?;
        Ok(ActiveState::from(response))
    }
}
