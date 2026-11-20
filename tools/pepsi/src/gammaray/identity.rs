use std::collections::HashSet;

use color_eyre::{
    Result,
    eyre::{bail, eyre},
};
use repository::team::{Robot, Team};

pub(super) fn robot_by_serial<'a>(team: &'a Team, serial: &str) -> Result<&'a Robot> {
    let mut numbers = HashSet::new();
    let mut serials = HashSet::new();
    for robot in &team.robots {
        if !numbers.insert(robot.number) {
            bail!("duplicate robot number {} in team.toml", robot.number);
        }
        if !serials.insert(&robot.id) {
            bail!("duplicate robot serial {:?} in team.toml", robot.id);
        }
    }
    team.robots
        .iter()
        .find(|robot| robot.id == serial)
        .ok_or_else(|| eyre!("robot serial {serial:?} not found in team.toml"))
}

pub(super) fn render(number: u8) -> String {
    format!(
        "ROS_Z_NAMESPACE=/{number}
ZENOH_SESSION_CONFIG_URI=/etc/zenoh/robot-peer.json5
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn team() -> Team {
        Team {
            team_number: 24,
            robots: vec![
                Robot {
                    number: 42,
                    hostname: "robot42".into(),
                    id: "123456".into(),
                },
                Robot {
                    number: 43,
                    hostname: "robot43".into(),
                    id: "654321".into(),
                },
            ],
        }
    }

    #[test]
    fn physical_serial_selects_the_identity() {
        let team = team();
        let robot = robot_by_serial(&team, "654321").unwrap();
        assert_eq!(robot.number, 43);
        assert_eq!(
            render(robot.number),
            "ROS_Z_NAMESPACE=/43\nZENOH_SESSION_CONFIG_URI=/etc/zenoh/robot-peer.json5\n"
        );
        assert!(robot_by_serial(&team, "unknown").is_err());
    }

    #[test]
    fn ambiguous_team_cannot_assign_any_identity() {
        let mut team = team();
        team.robots[1].number = 42;
        assert!(robot_by_serial(&team, "123456").is_err());

        team.robots[1].number = 43;
        team.robots[1].id = "123456".into();
        assert!(robot_by_serial(&team, "123456").is_err());
    }
}
