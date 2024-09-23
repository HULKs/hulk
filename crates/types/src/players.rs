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
