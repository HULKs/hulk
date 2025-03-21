use std::fmt::Display;

use regex::Regex;
use serde::{Deserialize, Serialize};
use zbus::{
    zvariant::{OwnedObjectPath, Value},
    Connection, Error, Proxy,
};

#[derive(Debug)]
enum Service {
    Hal,
    Hula,
    Hulk,
    Lola,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ServiceState {
    Activating,
    Active,
    Deactivating,
    Failed,
    Inactive,
    NotLoaded,
    Reloading,
    Unknown,
}

impl From<&str> for ServiceState {
    fn from(string: &str) -> Self {
        match string {
            "activating" => ServiceState::Activating,
            "active" => ServiceState::Active,
            "deactivating" => ServiceState::Deactivating,
            "failed" => ServiceState::Failed,
            "inactive" => ServiceState::Inactive,
            "reloading" => ServiceState::Reloading,
            _ => ServiceState::Unknown,
        }
    }
}

impl Display for ServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceState::Activating => write!(f, "Activating"),
            ServiceState::Active => write!(f, "Active"),
            ServiceState::Deactivating => write!(f, "Deactivating"),
            ServiceState::Failed => write!(f, "Failed"),
            ServiceState::Inactive => write!(f, "Inactive"),
            ServiceState::NotLoaded => write!(f, "NotLoaded"),
            ServiceState::Reloading => write!(f, "Reloading"),
            ServiceState::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct SystemServices {
    pub hal: ServiceState,
    pub hula: ServiceState,
    pub hulk: ServiceState,
    pub lola: ServiceState,
}

impl SystemServices {
    pub async fn query(dbus_connection: &Connection) -> Result<Self, Error> {
        Ok(Self {
            hal: get_service_state(dbus_connection, Service::Hal).await?,
            hula: get_service_state(dbus_connection, Service::Hula).await?,
            hulk: get_service_state(dbus_connection, Service::Hulk).await?,
            lola: get_service_state(dbus_connection, Service::Lola).await?,
        })
    }
}

async fn get_unit_path(
    dbus_connection: &Connection,
    service: Service,
) -> Result<OwnedObjectPath, Error> {
    let service_name = match service {
        Service::Hal => "hal.service",
        Service::Hula => "hula.service",
        Service::Hulk => "hulk.service",
        Service::Lola => "lola.service",
    };

    let proxy = Proxy::new(
        dbus_connection,
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .await?;

    proxy.call("GetUnit", &(service_name,)).await
}

async fn get_service_state(
    dbus_connection: &Connection,
    service: Service,
) -> Result<ServiceState, Error> {
    let regex = Regex::new(r"Unit \w+\.service not loaded").unwrap();

    let unit_path = match get_unit_path(dbus_connection, service).await {
        Ok(unit_path) => unit_path,
        Err(Error::MethodError(_, Some(message), _)) if regex.is_match(&message) => {
            return Ok(ServiceState::NotLoaded);
        }
        Err(error) => return Err(error),
    };

    let proxy = Proxy::new(
        dbus_connection,
        "org.freedesktop.systemd1",
        unit_path,
        "org.freedesktop.DBus.Properties",
    )
    .await?;

    if let Value::Str(state) = proxy
        .call_method("Get", &("org.freedesktop.systemd1.Unit", "ActiveState"))
        .await?
        .body()
        .deserialize()?
    {
        Ok(ServiceState::from(state.as_str()))
    } else {
        Err(Error::Failure("failed to get state".to_owned()))
    }
}
