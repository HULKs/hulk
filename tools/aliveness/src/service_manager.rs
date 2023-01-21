use color_eyre::eyre::{bail, Result, WrapErr};
use regex::Regex;
use serde::Serialize;
use zbus::{
    zvariant::{OwnedObjectPath, Value},
    Connection, Error, Proxy,
};

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

impl From<&str> for ActiveState {
    fn from(string: &str) -> Self {
        match string {
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
    pub async fn query(dbus_conn: &Connection) -> Result<Self> {
        Ok(Self {
            hal: get_service_state(dbus_conn, Service::Hal).await?,
            hula: get_service_state(dbus_conn, Service::Hula).await?,
            hulk: get_service_state(dbus_conn, Service::Hulk).await?,
            lola: get_service_state(dbus_conn, Service::Lola).await?,
        })
    }
}

async fn get_unit_path(
    dbus_conn: &Connection,
    service: Service,
) -> Result<OwnedObjectPath, zbus::Error> {
    let service_name = match service {
        Service::Hal => "hal.service",
        Service::Hula => "hula.service",
        Service::Hulk => "hulk.service",
        Service::Lola => "lola.service",
    };

    let proxy = Proxy::new(
        dbus_conn,
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .await?;

    proxy.call("GetUnit", &(service_name,)).await
}

async fn get_service_state(dbus_conn: &Connection, service: Service) -> Result<ActiveState> {
    let regex = Regex::new(r"Unit \w+\.service not loaded").unwrap();

    let unit_path = match get_unit_path(dbus_conn, service).await {
        Ok(unit_path) => unit_path,
        Err(Error::MethodError(_, Some(msg), _)) if regex.is_match(&msg) => {
            return Ok(ActiveState::NotLoaded);
        }
        Err(err) => {
            return Err(err).wrap_err("failed to unit path");
        }
    };

    let proxy = Proxy::new(
        dbus_conn,
        "org.freedesktop.systemd1",
        unit_path,
        "org.freedesktop.DBus.Properties",
    )
    .await?;

    if let Value::Str(state) = proxy
        .call_method("Get", &("org.freedesktop.systemd1.Unit", "ActiveState"))
        .await?
        .body()?
    {
        Ok(ActiveState::from(state.as_str()))
    } else {
        bail!("failed to get state")
    }
}
