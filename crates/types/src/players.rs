use std::ops::{Index, IndexMut};

use color_eyre::Result;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use spl_network_messages::{Penalty, PlayerNumber, TeamState};

#[derive(Clone, Copy, Default, Debug, Deserialize, Serialize, SerializeHierarchy)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct Players<T> {
    pub one: T,
    pub two: T,
    pub three: T,
    pub four: T,
    pub five: T,
    pub six: T,
    pub seven: T,
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
            PlayerNumber::Six => &self.six,
            PlayerNumber::Seven => &self.seven,
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
            PlayerNumber::Six => &mut self.six,
            PlayerNumber::Seven => &mut self.seven,
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
            six: team_state.players[5].penalty,
            seven: team_state.players[6].penalty,
        }
    }
}

#[derive(Clone, Copy)]
pub struct PlayersIterator<'a, T> {
    data: &'a Players<T>,
    next_forward: Option<PlayerNumber>,
    next_back: Option<PlayerNumber>,
}

impl<'a, T> PlayersIterator<'a, T> {
    fn new(data: &'a Players<T>) -> Self {
        Self {
            data,
            next_forward: Some(PlayerNumber::One),
            next_back: Some(PlayerNumber::Seven),
        }
    }
}

impl<'a, T> Iterator for PlayersIterator<'a, T> {
    type Item = (PlayerNumber, &'a T);
    fn next(&mut self) -> Option<Self::Item> {
        let result = self.next_forward.map(|number| (number, &self.data[number]));
        if self.next_forward == self.next_back {
            self.next_forward = None;
            self.next_back = None;
        }
        self.next_forward = match self.next_forward {
            Some(PlayerNumber::One) => Some(PlayerNumber::Two),
            Some(PlayerNumber::Two) => Some(PlayerNumber::Three),
            Some(PlayerNumber::Three) => Some(PlayerNumber::Four),
            Some(PlayerNumber::Four) => Some(PlayerNumber::Five),
            Some(PlayerNumber::Five) => Some(PlayerNumber::Six),
            Some(PlayerNumber::Six) => Some(PlayerNumber::Seven),
            Some(PlayerNumber::Seven) => None,
            None => None,
        };
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let consumed_forward = match self.next_forward {
            Some(PlayerNumber::One) => 0,
            Some(PlayerNumber::Two) => 1,
            Some(PlayerNumber::Three) => 2,
            Some(PlayerNumber::Four) => 3,
            Some(PlayerNumber::Five) => 4,
            Some(PlayerNumber::Six) => 5,
            Some(PlayerNumber::Seven) => 6,
            None => 7,
        };
        let consumed_back = match self.next_back {
            Some(PlayerNumber::One) => 6,
            Some(PlayerNumber::Two) => 5,
            Some(PlayerNumber::Three) => 4,
            Some(PlayerNumber::Four) => 3,
            Some(PlayerNumber::Five) => 2,
            Some(PlayerNumber::Six) => 1,
            Some(PlayerNumber::Seven) => 0,
            None => 7,
        };
        let remaining = 7usize.saturating_sub(consumed_forward + consumed_back);
        (remaining, Some(remaining))
    }
}

impl<'a, T> DoubleEndedIterator for PlayersIterator<'a, T>
where
    T: SerializeHierarchy,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let result = self.next_back.map(|number| (number, &self.data[number]));
        if self.next_forward == self.next_back {
            self.next_forward = None;
            self.next_back = None;
        }
        self.next_back = match self.next_back {
            Some(PlayerNumber::One) => None,
            Some(PlayerNumber::Two) => Some(PlayerNumber::One),
            Some(PlayerNumber::Three) => Some(PlayerNumber::Two),
            Some(PlayerNumber::Four) => Some(PlayerNumber::Three),
            Some(PlayerNumber::Five) => Some(PlayerNumber::Four),
            Some(PlayerNumber::Six) => Some(PlayerNumber::Five),
            Some(PlayerNumber::Seven) => Some(PlayerNumber::Six),
            None => None,
        };
        result
    }
}

impl<'a, T> ExactSizeIterator for PlayersIterator<'a, T>
where
    T: SerializeHierarchy,
{
    // The default implementation only requires `Iterator::size_hint()` to be exact
}

impl<T> Players<T>
where
    T: SerializeHierarchy,
{
    pub fn iter(&self) -> PlayersIterator<'_, T> {
        PlayersIterator::new(self)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn exact_size() {
        let players = Players::<i32>::default();
        let mut iterator = players.iter();

        assert_eq!(iterator.len(), 7);
        iterator.next();
        assert_eq!(iterator.len(), 6);
        iterator.next();
        assert_eq!(iterator.len(), 5);
        iterator.next();
        assert_eq!(iterator.len(), 4);
        iterator.next();
        assert_eq!(iterator.len(), 3);
        iterator.next();
        assert_eq!(iterator.len(), 2);
        iterator.next();
        assert_eq!(iterator.len(), 1);
        iterator.next();
        assert_eq!(iterator.len(), 0);
        iterator.next();
        assert_eq!(iterator.len(), 0);
        iterator.next();
    }

    #[test]
    fn double_ended() {
        let players = Players {
            one: 1,
            two: 2,
            three: 3,
            four: 4,
            five: 5,
            six: 6,
            seven: 7,
        };
        let mut iterator = players.iter();

        assert_eq!(iterator.len(), 7);
        assert_eq!(iterator.next(), Some((PlayerNumber::One, &1)));
        assert_eq!(iterator.len(), 6);
        assert_eq!(iterator.next(), Some((PlayerNumber::Two, &2)));
        assert_eq!(iterator.len(), 5);
        assert_eq!(iterator.next_back(), Some((PlayerNumber::Seven, &7)));
        assert_eq!(iterator.len(), 4);
        assert_eq!(iterator.next_back(), Some((PlayerNumber::Six, &6)));
        assert_eq!(iterator.len(), 3);
        assert_eq!(iterator.next(), Some((PlayerNumber::Three, &3)));
        assert_eq!(iterator.len(), 2);
        assert_eq!(iterator.next(), Some((PlayerNumber::Four, &4)));
        assert_eq!(iterator.len(), 1);
        assert_eq!(iterator.next_back(), Some((PlayerNumber::Five, &5)));
        assert_eq!(iterator.len(), 0);
        assert_eq!(iterator.next(), None);
        assert_eq!(iterator.len(), 0);
        assert_eq!(iterator.next_back(), None);
        assert_eq!(iterator.len(), 0);
        assert_eq!(iterator.next(), None);
        assert_eq!(iterator.len(), 0);
        assert_eq!(iterator.next_back(), None);
        assert_eq!(iterator.len(), 0);
    }
}
