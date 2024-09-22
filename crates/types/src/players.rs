use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut, Index, IndexMut};

use path_serde::{deserialize, serialize, PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use spl_network_messages::bindings::MAX_NUM_PLAYERS;
use spl_network_messages::{Penalty, TeamState};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Players<T> {
    pub inner: HashMap<usize, T>,
}
impl<T> Players<T> {
    pub fn new() -> Self {
        Players {
            inner: HashMap::with_capacity(MAX_NUM_PLAYERS as usize),
        }
    }

    pub fn new_with_content(content: T) -> Self
    where
        T: Clone,
    {
        let mut inner = HashMap::with_capacity(MAX_NUM_PLAYERS as usize);
        for i in 1..=MAX_NUM_PLAYERS as usize {
            inner.insert(i, content.clone());
        }
        Players { inner }
    }

    pub fn inner(&self) -> &HashMap<usize, T> {
        &self.inner
    }
}
impl<T> Default for Players<T>
where
    T: Default,
{
    fn default() -> Self {
        let mut inner = HashMap::with_capacity(MAX_NUM_PLAYERS as usize);
        for i in 1..=MAX_NUM_PLAYERS as usize {
            inner.insert(i, T::default());
        }
        Players { inner }
    }
}

impl<T> Deref for Players<T> {
    type Target = HashMap<usize, T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Players<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> Index<usize> for Players<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.inner.get(&index).expect("Players index out of bounds")
    }
}

impl<T> IndexMut<usize> for Players<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.inner
            .get_mut(&index)
            .expect("Players index out of bounds")
    }
}

impl From<TeamState> for Players<Option<Penalty>> {
    fn from(team_state: TeamState) -> Self {
        let mut inner = HashMap::with_capacity(MAX_NUM_PLAYERS as usize);

        for (i, player) in team_state.players.iter().enumerate() {
            if i < MAX_NUM_PLAYERS as usize {
                inner.insert(i, player.penalty);
            }
        }

        Players { inner }
    }
}

// impl<T> Serialize for Players<T>
// where
//     T: Serialize,
// {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         self.inner.serialize(serializer)
//     }
// }

// impl<'de, T> Deserialize<'de> for Players<T>
// where
//     T: Deserialize<'de>,
// {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let inner = HashMap::deserialize(deserializer)?;
//         if inner.len() > MAX_NUM_PLAYERS as usize {
//             return Err(serde::de::Error::custom("Too many players"));
//         }
//         Ok(Players { inner })
//     }
// }

impl<T> PathIntrospect for Players<T>
where
    T: PathIntrospect,
{
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
        for i in 1..=MAX_NUM_PLAYERS {
            fields.insert(format!("{}{}", prefix, i));
        }
    }
}

impl<T> PathSerialize for Players<T>
where
    T: PathSerialize + Serialize,
{
    fn serialize_path<S>(
        &self,
        path: &str,
        serializer: S,
    ) -> Result<S::Ok, serialize::Error<S::Error>>
    where
        S: Serializer,
    {
        let split = path.split_once('.');
        match (path, split) {
            (_, Some((index, suffix))) => {
                let index: usize =
                    index
                        .parse()
                        .map_err(|_| serialize::Error::PathDoesNotExist {
                            path: path.to_owned(),
                        })?;
                self.index(index).serialize_path(suffix, serializer)
            }
            (index, None) => {
                let index: usize =
                    index
                        .parse()
                        .map_err(|_| serialize::Error::PathDoesNotExist {
                            path: path.to_owned(),
                        })?;
                self.index(index)
                    .serialize(serializer)
                    .map_err(serialize::Error::SerializationFailed)
            }
        }
    }
}

impl<T> PathDeserialize for Players<T>
where
    T: PathDeserialize,
    for<'de> T: Deserialize<'de>,
{
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        let split = path.split_once('.');
        match (path, split) {
            (_, Some((index, suffix))) => {
                let index: usize =
                    index
                        .parse()
                        .map_err(|_| deserialize::Error::PathDoesNotExist {
                            path: path.to_owned(),
                        })?;
                self.index_mut(index).deserialize_path(suffix, deserializer)
            }
            (index, None) => {
                let index: usize =
                    index
                        .parse()
                        .map_err(|_| deserialize::Error::PathDoesNotExist {
                            path: path.to_owned(),
                        })?;
                let deserialized = T::deserialize(deserializer)
                    .map_err(deserialize::Error::DeserializationFailed)?;
                self.inner.insert(index, deserialized);
                Ok(())
            }
        }
    }
}
// impl<T: PathSerialize> Players<T>
// where
//     T: Serialize + PathSerialize,
// {
//     fn serialize_path<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let mut map = serializer.serialize_map(Some(self.inner.len()))?;
//         // .map_err(serialize::Error::SerializationFailed);
//         // Ok(map)
//         // self.inner.serialize_path(path, serializer)

//         for (key, value) in &self.inner {
//             map.serialize_entry(&key.to_string(), value)?;
//         }
//         map.end()
//     }
// }

// impl<T> PathDeserialize for Players<T>
// where
//     for<'de> T: Deserialize<'de>,
// {
//     fn deserialize_path<'de, D>(
//         &mut self,
//         path: &str,
//         deserializer: D,
//     ) -> Result<(), deserialize::Error<D::Error>>
//     where
//         D: Deserializer<'de>,
//     {
//         let map: HashMap<String, T> = HashMap::deserialize(deserializer)?;
//         self.inner = map
//             .into_iter()
//             .map(|(k, v)| {
//                 (
//                     k.parse::<usize>()
//                         .map_err(deserialize::Error::DeserializationFailed),
//                     v,
//                 )
//             })
//             .collect()?;
//         Ok(())
//     }
// }
// impl<'de, T: Deserialize<'de>> Players<T> {
//     pub fn deserialize_path<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let map: HashMap<String, T> = HashMap::deserialize(deserializer)?;
//         let inner: HashMap<usize, T> = map
//             .into_iter()
//             .map(|(k, v)| {
//                 (
//                     k.parse::<usize>()
//                         .map_err(deserialize::Error::DeserializationFailed),
//                     v,
//                 )
//             })
//             .collect()?;
//         Ok(Players { inner })
//     }
// }
// pub struct Players<T> {
//     pub inner: Vec<T>,
// }
// impl<T> Players<T> {
//     pub fn new() -> Self {
//         Players { inner: Vec::new() }
//     }
//     pub fn inner(&self) -> &Vec<T> {
//         &self.inner
//     }
//     pub fn new_with_size(size: usize) -> Self {
//         Players {
//             inner: Vec::with_capacity(size),
//         }
//     }
// }

// impl<T> Deref for Players<T> {
//     type Target = Vec<T>;

//     fn deref(&self) -> &Self::Target {
//         &self.inner
//     }
// }

// impl<T> Index<usize> for Players<T> {
//     type Output = T;

//     fn index(&self, index: usize) -> &Self::Output {
//         self.inner
//             .get(index - 1)
//             .expect("Players index out of bounds")
//     }
// }

// impl<T> IndexMut<usize> for Players<T> {
//     fn index_mut(&mut self, index: usize) -> &mut Self::Output {
//         &mut self.inner[index - 1]
//     }
// }
// impl<T> PathSerialize for Players<T> {
//     fn serialize_path<S>(
//         &self,
//         path: &str,
//         serializer: S,
//     ) -> Result<S::Ok, serialize::Error<S::Error>>
//     where
//         S: serde::Serializer,
//     {
//         print!("{:?}", path);
//         self.inner.serialize_path("1", serializer)
//     }
// }
// impl<T> PathDeserialize for Players<T> {
//     fn deserialize_path<'de, D>(
//         &mut self,
//         path: &str,
//         deserializer: D,
//     ) -> Result<(), deserialize::Error<D::Error>>
//     where
//         D: Deserializer<'de>,
//     {
//         print!("{:?}", path);
//         self.inner.deserialize_path(, deserializer)
//     }
// }
// impl<T> PathSerialize for Players<T>
// where
//     T: PathSerialize,
// {
//     fn serialize_path<S>(
//         &self,
//         path: &str,
//         serializer: S,
//     ) -> Result<S::Ok, serialize::Error<S::Error>>
//     where
//         S: Serializer,
//     {
//         let segments: Vec<&str> = path.split('.').collect();

//         // If the path is just "inner", serialize the whole vector
//         if segments.len() == 1 && segments[0] == "inner" {
//             return self.inner.serialize_path(path, serializer);
//         }
//         if segments.len() == 2 && segments[0] == "inner" {
//             if let Ok(index) = segments[1].parse::<usize>() {
//                 if let Some(value) = self.inner.get(index) {
//                     return value.serialize_path("", serializer);
//                 } else {
//                     return Err(serialize::Error::PathDoesNotExist {
//                         path: index.to_string(),
//                     });
//                 }
//             }
//         }
//         // // Attempt to parse the path as an integer index
//         // if let Ok(index) = path.parse::<usize>() {
//         //     if let Some(value) = self.inner.get(index) {
//         //         return value.serialize_path("", serializer);
//         //     } else {
//         //         return Err(serialize::Error::PathDoesNotExist {
//         //             path: index.to_string(),
//         //         });
//         //     }
//         // }

//         Err(serialize::Error::PathDoesNotExist {
//             path: path.to_owned(),
//         })
//     }
// }

// impl<T> PathDeserialize for Players<T>
// where
//     T: PathDeserialize + Deserialize,
// {
//     fn deserialize_path<'de, D>(
//         &mut self,
//         path: &str,
//         deserializer: D,
//     ) -> Result<(), deserialize::Error<D::Error>>
//     where
//         D: Deserializer<'de>,
//     {
//         let segments: Vec<&str> = path.split('.').collect();

//         // If the path is just "inner", deserialize the whole vector
//         if segments.len() == 1 && segments[0] == "inner" {
//             self.inner.serialize_path("", deserializer);
//             return Ok(());
//         }

//         // // Attempt to parse the path as an integer index
//         // if let Ok(index) = path.parse::<usize>() {
//         //     if index >= self.inner.len() {
//         //         return Err(deserialize::Error::PathDoesNotExist {
//         //             path: index.to_string(),
//         //         });
//         //     }

//         //     self.inner[index].deserialize_path("", deserializer)?;
//         //     return Ok(());
//         // }

//         Err(deserialize::Error::PathDoesNotExist {
//             path: path.to_owned(),
//         })
//     }
// }
// impl<T> PathSerialize for Players<T>
// where
//     T: PathSerialize + Serialize,
// {
//     fn serialize_path<S>(
//         &self,
//         path: &str,
//         serializer: S,
//     ) -> Result<S::Ok, path_serde::serialize::Error<S::Error>>
//     where
//         S: serde::Serializer,
//     {
//         if path.is_empty() {
//             return self.serialize(serializer).map_err(PathError::custom);
//         }

//         // Handle path indexing like `inner.N`
//         match path[0] {
//             PathFragment::Index(index) => {
//                 if let Some(value) = self.inner.get(index) {
//                     return value.path_serialize(&path[1..], serializer);
//                 }
//                 Err(PathError::custom("Index out of bounds"))
//             }
//             _ => Err(PathError::custom("Invalid path")),
//         }
//     }
// }
// impl<'de, T> PathDeserialize for Players<T>
// where
//     T: PathDeserialize + Deserialize<'de>,
// {
//     fn deserialize_path<D>(
//         &mut self,
//         path: &str,
//         deserializer: D,
//     ) -> Result<(), path_serde::deserialize::Error<D::Error>>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         if path.is_empty() {
//             return Ok(*self = Deserialize::deserialize(deserializer).map_err(PathError::custom));
//         }

//         // Handle path indexing like `inner.N`
//         match path[0] {
//             PathFragment::Index(index) => {
//                 if let Some(value) = self.inner.get_mut(index) {
//                     return value.path_deserialize(&path[1..], deserializer);
//                 }
//                 Err(PathError::custom("Index out of bounds"))
//             }
//             _ => Err(PathError::custom("Invalid path")),
//         }
//     }
// }
// // Implement PathSerialize and PathDeserialize for indexed access
// impl<T> PathSerialize for Players<T>
// where
//     T: PathSerialize + Serialize,
// {
//     fn path_serialize(
//         &self,
//         path: &[PathFragment],
//         serializer: &mut dyn erased_serde::Serializer,
//     ) -> Result<(), PathError> {
//         if path.is_empty() {
//             return self.serialize(serializer).map_err(PathError::custom);
//         }

//         // Handle path indexing like `inner.N`
//         match path[0] {
//             PathFragment::Index(index) => {
//                 if let Some(value) = self.inner.get(index) {
//                     return value.path_serialize(&path[1..], serializer);
//                 }
//                 Err(PathError::custom("Index out of bounds"))
//             }
//             _ => Err(PathError::custom("Invalid path")),
//         }
//     }
// }

// impl<'de, T> PathDeserialize<'de> for Players<T>
// where
//     T: PathDeserialize<'de> + Deserialize<'de>,
// {
//     fn path_deserialize(
//         &mut self,
//         path: &[PathFragment],
//         deserializer: &mut dyn erased_serde::Deserializer<'de>,
//     ) -> Result<(), PathError> {
//         if path.is_empty() {
//             return *self = Deserialize::deserialize(deserializer).map_err(PathError::custom);
//         }

//         // Handle path indexing like `inner.N`
//         match path[0] {
//             PathFragment::Index(index) => {
//                 if let Some(value) = self.inner.get_mut(index) {
//                     return value.path_deserialize(&path[1..], deserializer);
//                 }
//                 Err(PathError::custom("Index out of bounds"))
//             }
//             _ => Err(PathError::custom("Invalid path")),
//         }
//     }
// }

// use std::ops::{Index, IndexMut};

// use color_eyre::Result;
// use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
// use serde::{Deserialize, Serialize};
// use spl_network_messages::{Penalty, PlayerNumber, TeamState};

// #[derive(
//     Clone,
//     Copy,
//     Default,
//     Debug,
//     Deserialize,
//     Serialize,
//     PathSerialize,
//     PathIntrospect,
//     PathDeserialize,
//     PartialEq,
// )]

// pub struct Players<T> {
//     pub one: T,
//     pub two: T,
//     pub three: T,
//     pub four: T,
//     pub five: T,
//     pub six: T,
//     pub seven: T,
// }

// impl<T> Index<PlayerNumber> for Players<T> {
//     type Output = T;

//     fn index(&self, index: PlayerNumber) -> &Self::Output {
//         match index {
//             PlayerNumber::One => &self.one,
//             PlayerNumber::Two => &self.two,
//             PlayerNumber::Three => &self.three,
//             PlayerNumber::Four => &self.four,
//             PlayerNumber::Five => &self.five,
//             PlayerNumber::Six => &self.six,
//             PlayerNumber::Seven => &self.seven,
//         }
//     }
// }

// fn get_penalty(team_state: &TeamState, index: usize) -> Option<Penalty> {
//     team_state
//         .players
//         .get(index)
//         .map(|player| player.penalty)
//         .unwrap_or_default()
// }

// impl From<TeamState> for Players<Option<Penalty>> {
//     fn from(team_state: TeamState) -> Self {
//         Self {
//             one: get_penalty(&team_state, 0),
//             two: get_penalty(&team_state, 1),
//             three: get_penalty(&team_state, 2),
//             four: get_penalty(&team_state, 3),
//             five: get_penalty(&team_state, 4),
//             six: get_penalty(&team_state, 5),
//             seven: get_penalty(&team_state, 6),
//         }
//     }
// }

// #[derive(Clone, Copy)]
// pub struct PlayersIterator<'a, T> {
//     data: &'a Players<T>,
//     next_forward: Option<PlayerNumber>,
//     next_back: Option<PlayerNumber>,
// }

// impl<'a, T> PlayersIterator<'a, T> {
//     fn new(data: &'a Players<T>) -> Self {
//         Self {
//             data,
//             next_forward: Some(PlayerNumber::One),
//             next_back: Some(PlayerNumber::Seven),
//         }
//     }
// }

// impl<'a, T> Iterator for PlayersIterator<'a, T> {
//     type Item = (PlayerNumber, &'a T);
//     fn next(&mut self) -> Option<Self::Item> {
//         let result = self.next_forward.map(|number| (number, &self.data[number]));
//         if self.next_forward == self.next_back {
//             self.next_forward = None;
//             self.next_back = None;
//         }
//         self.next_forward = match self.next_forward {
//             Some(PlayerNumber::One) => Some(PlayerNumber::Two),
//             Some(PlayerNumber::Two) => Some(PlayerNumber::Three),
//             Some(PlayerNumber::Three) => Some(PlayerNumber::Four),
//             Some(PlayerNumber::Four) => Some(PlayerNumber::Five),
//             Some(PlayerNumber::Five) => Some(PlayerNumber::Six),
//             Some(PlayerNumber::Six) => Some(PlayerNumber::Seven),
//             Some(PlayerNumber::Seven) => None,
//             None => None,
//         };
//         result
//     }

//     fn size_hint(&self) -> (usize, Option<usize>) {
//         let consumed_forward = match self.next_forward {
//             Some(PlayerNumber::One) => 0,
//             Some(PlayerNumber::Two) => 1,
//             Some(PlayerNumber::Three) => 2,
//             Some(PlayerNumber::Four) => 3,
//             Some(PlayerNumber::Five) => 4,
//             Some(PlayerNumber::Six) => 5,
//             Some(PlayerNumber::Seven) => 6,
//             None => 7,
//         };
//         let consumed_back = match self.next_back {
//             Some(PlayerNumber::One) => 6,
//             Some(PlayerNumber::Two) => 5,
//             Some(PlayerNumber::Three) => 4,
//             Some(PlayerNumber::Four) => 3,
//             Some(PlayerNumber::Five) => 2,
//             Some(PlayerNumber::Six) => 1,
//             Some(PlayerNumber::Seven) => 0,
//             None => 7,
//         };
//         let remaining = 7usize.saturating_sub(consumed_forward + consumed_back);
//         (remaining, Some(remaining))
//     }
// }

// impl<'a, T> DoubleEndedIterator for PlayersIterator<'a, T> {
//     fn next_back(&mut self) -> Option<Self::Item> {
//         let result = self.next_back.map(|number| (number, &self.data[number]));
//         if self.next_forward == self.next_back {
//             self.next_forward = None;
//             self.next_back = None;
//         }
//         self.next_back = match self.next_back {
//             Some(PlayerNumber::One) => None,
//             Some(PlayerNumber::Two) => Some(PlayerNumber::One),
//             Some(PlayerNumber::Three) => Some(PlayerNumber::Two),
//             Some(PlayerNumber::Four) => Some(PlayerNumber::Three),
//             Some(PlayerNumber::Five) => Some(PlayerNumber::Four),
//             Some(PlayerNumber::Six) => Some(PlayerNumber::Five),
//             Some(PlayerNumber::Seven) => Some(PlayerNumber::Six),
//             None => None,
//         };
//         result
//     }
// }

// impl<'a, T> ExactSizeIterator for PlayersIterator<'a, T> {
//     // The default implementation only requires `Iterator::size_hint()` to be exact
// }

// impl<T> Players<T> {
//     pub fn iter(&self) -> PlayersIterator<'_, T> {
//         PlayersIterator::new(self)
//     }
// }

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     fn exact_size() {
//         let players = Players::<i32>::default();
//         let mut iterator = players.iter();

//         assert_eq!(iterator.len(), 7);
//         iterator.next();
//         assert_eq!(iterator.len(), 6);
//         iterator.next();
//         assert_eq!(iterator.len(), 5);
//         iterator.next();
//         assert_eq!(iterator.len(), 4);
//         iterator.next();
//         assert_eq!(iterator.len(), 3);
//         iterator.next();
//         assert_eq!(iterator.len(), 2);
//         iterator.next();
//         assert_eq!(iterator.len(), 1);
//         iterator.next();
//         assert_eq!(iterator.len(), 0);
//         iterator.next();
//         assert_eq!(iterator.len(), 0);
//         iterator.next();
//     }

//     #[test]
//     fn double_ended() {
//         let players = Players {
//             one: 1,
//             two: 2,
//             three: 3,
//             four: 4,
//             five: 5,
//             six: 6,
//             seven: 7,
//         };
//         let mut iterator = players.iter();

//         assert_eq!(iterator.len(), 7);
//         assert_eq!(iterator.next(), Some((PlayerNumber::One, &1)));
//         assert_eq!(iterator.len(), 6);
//         assert_eq!(iterator.next(), Some((PlayerNumber::Two, &2)));
//         assert_eq!(iterator.len(), 5);
//         assert_eq!(iterator.next_back(), Some((PlayerNumber::Seven, &7)));
//         assert_eq!(iterator.len(), 4);
//         assert_eq!(iterator.next_back(), Some((PlayerNumber::Six, &6)));
//         assert_eq!(iterator.len(), 3);
//         assert_eq!(iterator.next(), Some((PlayerNumber::Three, &3)));
//         assert_eq!(iterator.len(), 2);
//         assert_eq!(iterator.next(), Some((PlayerNumber::Four, &4)));
//         assert_eq!(iterator.len(), 1);
//         assert_eq!(iterator.next_back(), Some((PlayerNumber::Five, &5)));
//         assert_eq!(iterator.len(), 0);
//         assert_eq!(iterator.next(), None);
//         assert_eq!(iterator.len(), 0);
//         assert_eq!(iterator.next_back(), None);
//         assert_eq!(iterator.len(), 0);
//         assert_eq!(iterator.next(), None);
//         assert_eq!(iterator.len(), 0);
//         assert_eq!(iterator.next_back(), None);
//         assert_eq!(iterator.len(), 0);
//     }
// }
