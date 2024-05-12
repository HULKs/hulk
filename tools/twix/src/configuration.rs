use std::path::Path;

use serde::Deserialize;

mod keys;

#[derive(Debug, Deserialize)]
pub struct Configuration {
    pub keys: keys::Keybinds,
}

impl Configuration {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ()> {
        let config_file = r#"
            [keys]
            "C-t" = "open_split"
            "C-t" = "open_split"
        "#;

        let configuration: Configuration = toml::from_str(config_file).unwrap();

        Ok(configuration)
    }
}
