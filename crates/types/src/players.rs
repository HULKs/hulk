use std::collections::{BTreeMap, HashSet};
use std::ops::{Deref, DerefMut, Index, IndexMut};

use path_serde::{deserialize, serialize, PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use spl_network_messages::bindings::MAX_NUM_PLAYERS;
use spl_network_messages::{Penalty, TeamState};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Players<T> {
    pub inner: BTreeMap<u8, T>,
}
impl<T> Players<T> {
    pub fn new() -> Self {
        Players {
            inner: BTreeMap::new(),
        }
    }

    pub fn new_with_content(content: T) -> Self
    where
        T: Clone,
    {
        let mut inner = BTreeMap::new();
        for i in 1..=MAX_NUM_PLAYERS {
            inner.insert(i, content.clone());
        }
        Players { inner }
    }

    pub fn inner(&self) -> &BTreeMap<u8, T> {
        &self.inner
    }
}
impl<T> Default for Players<T>
where
    T: Default,
{
    fn default() -> Self {
        let mut inner = BTreeMap::new();
        for i in 1..=MAX_NUM_PLAYERS {
            inner.insert(i, T::default());
        }
        Players { inner }
    }
}

impl<T> Deref for Players<T> {
    type Target = BTreeMap<u8, T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Players<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> Index<u8> for Players<T> {
    type Output = T;

    fn index(&self, index: u8) -> &Self::Output {
        self.inner.get(&index).expect("Players index out of bounds")
    }
}

impl<T> IndexMut<u8> for Players<T> {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        self.inner
            .get_mut(&index)
            .expect("Players index out of bounds")
    }
}

impl From<TeamState> for Players<Option<Penalty>> {
    fn from(team_state: TeamState) -> Self {
        let mut inner = BTreeMap::new();

        for (index, player) in team_state.players.iter().enumerate() {
            if let Ok(u8_index) = u8::try_from(index) {
                if u8_index < MAX_NUM_PLAYERS {
                    inner.insert(u8_index + 1, player.penalty);
                }
            }
        }

        Players { inner }
    }
}

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
                let index: u8 = index
                    .parse()
                    .map_err(|_| serialize::Error::PathDoesNotExist {
                        path: path.to_owned(),
                    })?;
                self.index(index).serialize_path(suffix, serializer)
            }
            (index, None) => {
                let index: u8 = index
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
                let index: u8 =
                    index
                        .parse()
                        .map_err(|_| deserialize::Error::PathDoesNotExist {
                            path: path.to_owned(),
                        })?;
                self.index_mut(index).deserialize_path(suffix, deserializer)
            }
            (index, None) => {
                let index: u8 =
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
