use std::{collections::HashMap, fmt};

use eframe::egui::{Key, Modifiers};
use serde::{
    de::{self, Deserializer},
    Deserialize,
};
use thiserror::Error;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Error)]
pub enum Error {
    #[error("duplicate modifier `{0}`")]
    DuplicateModifier(String),
    #[error("invalid modifier `{0}`")]
    InvalidModifier(String),
    #[error("invalid key `{0}`")]
    InvalidKey(String),
    #[error("unsupported keybind `{0}`")]
    UnsupportedKeybind(String),
}

// Make sure to update docs/tooling/twix.md when changing this!
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KeybindAction {
    CloseTab,
    DuplicateTab,
    FocusAbove,
    FocusAddress,
    FocusBelow,
    FocusLeft,
    FocusPanel,
    FocusRight,
    NoOp,
    OpenSplit,
    OpenTab,
    Reconnect,
}

#[derive(Debug, PartialEq, Eq, Hash)]
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

    fn is_supported_keybind(&self) -> bool {
        match self {
            // Binding CTRL+[cvx] is not supported.
            // See https://github.com/emilk/egui/issues/4065
            KeybindTrigger {
                key: Key::C | Key::V | Key::X,
                modifiers: Modifiers::COMMAND,
            } => false,
            _ => true,
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

        let result = Self { key, modifiers };

        if result.is_supported_keybind() {
            Ok(result)
        } else {
            Err(Error::UnsupportedKeybind(v.into()))
        }
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

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct Keybinds {
    keybinds: HashMap<KeybindTrigger, KeybindAction>,
}

impl Keybinds {
    pub fn new() -> Self {
        Self {
            keybinds: HashMap::new(),
        }
    }

    pub fn merge(&mut self, other: Self) {
        self.keybinds.extend(other.keybinds);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&KeybindTrigger, &KeybindAction)> {
        self.keybinds.iter()
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
                let mut keybinds = HashMap::new();

                while let Some((trigger, action)) = visitor.next_entry()? {
                    keybinds.insert(trigger, action);
                }

                Ok(Keybinds { keybinds })
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
            KeybindTrigger::parse("C-p"),
            Ok(KeybindTrigger {
                key: Key::P,
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

        assert_eq!(
            KeybindTrigger::parse("C-c"),
            Err(Error::UnsupportedKeybind("C-c".into()))
        );
    }
}
