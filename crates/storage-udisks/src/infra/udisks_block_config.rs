use std::collections::HashMap;

use zbus::proxy;
use zbus::zvariant::OwnedValue;

pub type ConfigurationItem = (String, HashMap<String, OwnedValue>);

#[proxy(
    interface = "org.freedesktop.UDisks2.Block",
    default_service = "org.freedesktop.UDisks2",
    assume_defaults = true
)]
pub trait UDisks2BlockConfiguration {
    #[zbus(property)]
    fn configuration(&self) -> zbus::Result<Vec<ConfigurationItem>>;

    /// Method signature (per introspection): (sa{sv}) a{sv}
    fn add_configuration_item(
        &self,
        item: ConfigurationItem,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;

    /// Method signature (per introspection): (sa{sv}) a{sv}
    fn remove_configuration_item(
        &self,
        item: ConfigurationItem,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;

    /// Method signature (per introspection): (sa{sv}) (sa{sv}) a{sv}
    fn update_configuration_item(
        &self,
        old_item: ConfigurationItem,
        new_item: ConfigurationItem,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;

    /// Method signature (per introspection): a{sv} -> a(sa{sv})
    fn get_secret_configuration(
        &self,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<Vec<ConfigurationItem>>;
}
