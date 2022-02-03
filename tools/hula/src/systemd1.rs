use dbus;
use dbus::arg;
use dbus::blocking;

pub trait OrgFreedesktopDBusProperties {
    fn get<R0: for<'b> arg::Get<'b> + 'static>(
        &self,
        interface: &str,
        property: &str,
    ) -> Result<R0, dbus::Error>;
    fn get_all(&self, interface: &str) -> Result<arg::PropMap, dbus::Error>;
    fn set<I2: arg::Arg + arg::Append>(
        &self,
        interface: &str,
        property: &str,
        value: I2,
    ) -> Result<(), dbus::Error>;
}

impl<'a, T: blocking::BlockingSender, C: ::std::ops::Deref<Target = T>> OrgFreedesktopDBusProperties
    for blocking::Proxy<'a, C>
{
    fn get<R0: for<'b> arg::Get<'b> + 'static>(
        &self,
        interface: &str,
        property: &str,
    ) -> Result<R0, dbus::Error> {
        self.method_call(
            "org.freedesktop.DBus.Properties",
            "Get",
            (interface, property),
        )
        .and_then(|r: (arg::Variant<R0>,)| Ok((r.0).0))
    }

    fn get_all(&self, interface: &str) -> Result<arg::PropMap, dbus::Error> {
        self.method_call("org.freedesktop.DBus.Properties", "GetAll", (interface,))
            .and_then(|r: (arg::PropMap,)| Ok(r.0))
    }

    fn set<I2: arg::Arg + arg::Append>(
        &self,
        interface: &str,
        property: &str,
        value: I2,
    ) -> Result<(), dbus::Error> {
        self.method_call(
            "org.freedesktop.DBus.Properties",
            "Set",
            (interface, property, arg::Variant(value)),
        )
    }
}

pub trait OrgFreedesktopSystemd1Manager {
    fn get_unit(&self, name: &str) -> Result<dbus::Path<'static>, dbus::Error>;
}

impl<'a, T: blocking::BlockingSender, C: ::std::ops::Deref<Target = T>>
    OrgFreedesktopSystemd1Manager for blocking::Proxy<'a, C>
{
    fn get_unit(&self, name: &str) -> Result<dbus::Path<'static>, dbus::Error> {
        self.method_call("org.freedesktop.systemd1.Manager", "GetUnit", (name,))
            .and_then(|r: (dbus::Path<'static>,)| Ok(r.0))
    }
}
