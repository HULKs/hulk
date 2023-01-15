use std::{
    collections::BTreeSet,
    iter::once,
    ops::{Index, IndexMut},
};

use color_eyre::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serialize_hierarchy::{Error, SerializeHierarchy, Serializer};
use spl_network_messages::{Penalty, PlayerNumber, TeamState};

#[derive(Clone, Copy, Default, Debug, Deserialize, Serialize)]
pub struct Players<T> {
    pub one: T,
    pub two: T,
    pub three: T,
    pub four: T,
    pub five: T,
}

impl<T> Index<PlayerNumber> for Players<T> {
    type Output = T;

    fn index(&self, index: PlayerNumber) -> &Self::Output {
        match index {
            PlayerNumber::One => &self.one,
            PlayerNumber::Two => &self.two,
            PlayerNumber::Three => &self.three,
            PlayerNumber::Four => &self.four,
            PlayerNumber::Five => &self.five,
        }
    }
}

impl<T> IndexMut<PlayerNumber> for Players<T> {
    fn index_mut(&mut self, index: PlayerNumber) -> &mut Self::Output {
        match index {
            PlayerNumber::One => &mut self.one,
            PlayerNumber::Two => &mut self.two,
            PlayerNumber::Three => &mut self.three,
            PlayerNumber::Four => &mut self.four,
            PlayerNumber::Five => &mut self.five,
        }
    }
}

impl From<TeamState> for Players<Option<Penalty>> {
    fn from(team_state: TeamState) -> Self {
        Self {
            one: team_state.players[0].penalty,
            two: team_state.players[1].penalty,
            three: team_state.players[2].penalty,
            four: team_state.players[3].penalty,
            five: team_state.players[4].penalty,
        }
    }
}

pub struct PlayersIterator<'a, T> {
    data: &'a Players<T>,
    player_number: Option<PlayerNumber>,
}

impl<'a, T> PlayersIterator<'a, T> {
    fn new(data: &'a Players<T>) -> Self {
        Self {
            data,
            player_number: Some(PlayerNumber::One),
        }
    }
}

impl<'a, T> Iterator for PlayersIterator<'a, T> {
    type Item = (PlayerNumber, &'a T);
    fn next(&mut self) -> Option<Self::Item> {
        let result = self.player_number.map(|number| match number {
            PlayerNumber::One => (PlayerNumber::One, &self.data.one),
            PlayerNumber::Two => (PlayerNumber::Two, &self.data.two),
            PlayerNumber::Three => (PlayerNumber::Three, &self.data.three),
            PlayerNumber::Four => (PlayerNumber::Four, &self.data.four),
            PlayerNumber::Five => (PlayerNumber::Five, &self.data.five),
        });
        self.player_number = match self.player_number {
            Some(PlayerNumber::One) => Some(PlayerNumber::Two),
            Some(PlayerNumber::Two) => Some(PlayerNumber::Three),
            Some(PlayerNumber::Three) => Some(PlayerNumber::Four),
            Some(PlayerNumber::Four) => Some(PlayerNumber::Five),
            Some(PlayerNumber::Five) => None,
            None => None,
        };
        result
    }
}

impl<T> Players<T> {
    pub fn iter(&self) -> PlayersIterator<'_, T> {
        PlayersIterator::new(self)
    }
}

impl<T> SerializeHierarchy for Players<T>
where
    T: Serialize + DeserializeOwned + SerializeHierarchy,
{
    fn serialize_path<S>(&self, path: &str) -> Result<S::Serialized, Error<S::Error>>
    where
        S: Serializer,
        S::Error: std::error::Error,
    {
        let split = path.split_once('.');
        match split {
            Some((name, suffix)) => match name {
                "one" => self.one.serialize_path::<S>(suffix),
                "two" => self.two.serialize_path::<S>(suffix),
                "three" => self.three.serialize_path::<S>(suffix),
                "four" => self.four.serialize_path::<S>(suffix),
                "five" => self.five.serialize_path::<S>(suffix),
                name => Err(Error::UnexpectedPathSegment {
                    segment: name.to_string(),
                }),
            },
            None => match path {
                "one" => S::serialize(&self.one).map_err(Error::SerializationFailed),
                "two" => S::serialize(&self.two).map_err(Error::SerializationFailed),
                "three" => S::serialize(&self.three).map_err(Error::SerializationFailed),
                "four" => S::serialize(&self.four).map_err(Error::SerializationFailed),
                "five" => S::serialize(&self.five).map_err(Error::SerializationFailed),
                name => Err(Error::UnexpectedPathSegment {
                    segment: name.to_string(),
                }),
            },
        }
    }

    fn deserialize_path<S>(
        &mut self,
        path: &str,
        data: S::Serialized,
    ) -> Result<(), Error<S::Error>>
    where
        S: Serializer,
        S::Error: std::error::Error,
    {
        let split = path.split_once('.');
        match split {
            Some((name, suffix)) => match name {
                "one" => self.one.deserialize_path::<S>(suffix, data),
                "two" => self.two.deserialize_path::<S>(suffix, data),
                "three" => self.three.deserialize_path::<S>(suffix, data),
                "four" => self.four.deserialize_path::<S>(suffix, data),
                "five" => self.five.deserialize_path::<S>(suffix, data),
                name => Err(Error::UnexpectedPathSegment {
                    segment: name.to_string(),
                }),
            },
            None => match path {
                "one" => {
                    self.one = S::deserialize(data).map_err(Error::DeserializationFailed)?;
                    Ok(())
                }
                "two" => {
                    self.two = S::deserialize(data).map_err(Error::DeserializationFailed)?;
                    Ok(())
                }
                "three" => {
                    self.three = S::deserialize(data).map_err(Error::DeserializationFailed)?;
                    Ok(())
                }
                "four" => {
                    self.four = S::deserialize(data).map_err(Error::DeserializationFailed)?;
                    Ok(())
                }
                "five" => {
                    self.five = S::deserialize(data).map_err(Error::DeserializationFailed)?;
                    Ok(())
                }
                name => Err(Error::UnexpectedPathSegment {
                    segment: name.to_string(),
                }),
            },
        }
    }

    fn exists(path: &str) -> bool {
        let split = path.split_once('.');
        match split {
            Some((name, suffix)) => match name {
                "one" => T::exists(suffix),
                "two" => T::exists(suffix),
                "three" => T::exists(suffix),
                "four" => T::exists(suffix),
                "five" => T::exists(suffix),
                _ => false,
            },
            None => matches!(path, "one" | "two" | "three" | "four" | "five"),
        }
    }

    fn get_fields() -> BTreeSet<String> {
        once(String::new())
            .chain(
                T::get_fields()
                    .into_iter()
                    .map(|name| format!("one.{name}")),
            )
            .chain(
                T::get_fields()
                    .into_iter()
                    .map(|name| format!("two.{name}")),
            )
            .chain(
                T::get_fields()
                    .into_iter()
                    .map(|name| format!("three.{name}")),
            )
            .chain(
                T::get_fields()
                    .into_iter()
                    .map(|name| format!("four.{name}")),
            )
            .chain(
                T::get_fields()
                    .into_iter()
                    .map(|name| format!("five.{name}")),
            )
            .collect()
    }
}
