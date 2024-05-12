use std::fmt;

use serde::{
    de::{self, Deserializer},
    Deserialize,
};

use eframe::egui::{Key, Modifiers};
use thiserror::Error;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Error)]
pub enum Error {
    #[error("Duplicate modifier `{0}`")]
    DuplicateModifier(String),
    #[error("Invalid modifier `{0}`")]
    InvalidModifier(String),
    #[error("Invalid key `{0}`")]
    InvalidKey(String),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeybindAction {
    OpenSplit,
    OpenTab,
    Reconnect,
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct KeybindTrigger {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl KeybindTrigger {
    pub fn parse_modifier(value: &&str) -> Result<Modifiers, Error> {
        match *value {
            "A" => Ok(Modifiers::ALT),
            "C" => Ok(Modifiers::COMMAND),
            "S" => Ok(Modifiers::SHIFT),
            _ => Err(Error::InvalidModifier(String::from(*value))),
        }
    }

    pub fn parse(v: &str) -> Result<Self, Error> {
        let parts = v.split('-').collect::<Vec<_>>();

        let Some((raw_key, raw_modifiers)) = parts.split_last() else {
            return Err(Error::InvalidKey(v.into()));
        };

        let is_single_ascii_uppercase_letter =
            matches!(raw_key.as_bytes(), [letter] if letter.is_ascii_uppercase());

        let Some(key) = Key::from_name(raw_key) else {
            return Err(Error::InvalidKey(v.into()));
        };

        let mut modifiers = Modifiers {
            shift: is_single_ascii_uppercase_letter,
            ..Modifiers::NONE
        };

        for raw_modifier in raw_modifiers {
            let modifier = KeybindTrigger::parse_modifier(raw_modifier)?;

            if modifiers.contains(modifier) {
                return Err(Error::DuplicateModifier(String::from(*raw_modifier)));
            }

            modifiers = modifiers | modifier;
        }

        Ok(Self { key, modifiers })
    }
}

impl<'de> Deserialize<'de> for KeybindTrigger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = KeybindTrigger;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                KeybindTrigger::parse(v).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[derive(Debug)]
pub struct Keybind {
    pub trigger: KeybindTrigger,
    pub action: KeybindAction,
}

#[derive(Debug)]
pub struct Keybinds {
    pub keys: Vec<Keybind>,
}

impl Keybinds {
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }
}

impl<'de> Deserialize<'de> for Keybinds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Keybinds;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Keybinds::new())
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut keys = Vec::new();

                while let Some((trigger, action)) = visitor.next_entry()? {
                    keys.push(Keybind { trigger, action });
                }

                Ok(Keybinds { keys })
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Key, Modifiers};

    use super::{Error, KeybindTrigger};

    #[test]
    fn parse_triggers() {
        assert_eq!(
            KeybindTrigger::parse("C-x"),
            Ok(KeybindTrigger {
                key: Key::X,
                modifiers: Modifiers::COMMAND
            })
        );

        assert_eq!(
            KeybindTrigger::parse("A-S-Esc"),
            Ok(KeybindTrigger {
                key: Key::Escape,
                modifiers: Modifiers::ALT | Modifiers::SHIFT
            })
        );

        assert_eq!(
            KeybindTrigger::parse("C-ArrowDown"),
            Ok(KeybindTrigger {
                key: Key::ArrowDown,
                modifiers: Modifiers::COMMAND
            })
        );

        assert_eq!(
            KeybindTrigger::parse("F1"),
            Ok(KeybindTrigger {
                key: Key::F1,
                modifiers: Modifiers::NONE
            })
        );

        assert_eq!(
            KeybindTrigger::parse("X-X"),
            Err(Error::InvalidModifier("X".into()))
        );

        assert_eq!(
            KeybindTrigger::parse("XX"),
            Err(Error::InvalidKey("XX".into()))
        );

        assert_eq!(
            KeybindTrigger::parse("S-A"),
            Err(Error::DuplicateModifier("S".into()))
        );

        assert_eq!(
            KeybindTrigger::parse("C-A-C-x"),
            Err(Error::DuplicateModifier("C".into()))
        );
    }
}
