use chrono::Duration;
use types::PrimaryState;

use crate::robot::Robot;

pub struct State {
    pub time_elapsed: Duration,
    pub robots: Vec<Robot>,
}

impl State {
    pub fn new(robot_count: usize) -> Self {
        let robots: Vec<_> = (0..robot_count).map(|index| Robot::new(index)).collect();

        Self {
            time_elapsed: Duration::zero(),
            robots,
        }
    }

    pub fn stiffen_robots(&mut self) {
        for robot in &mut self.robots {
            robot.primary_state = PrimaryState::Playing;
        }
    }

    pub fn cycle(&mut self) {
        for robot in &mut self.robots {
            println!("cycling");
            robot.cycle().unwrap();
            let database = robot.database.clone();
            println!("{:?}", database.main_outputs.motion_command);
            // match database.main_outputs.motion_command {
            //     types::MotionCommand::Walk {
            //         head,
            //         path,
            //         left_arm,
            //         right_arm,
            //         orientation_mode,
            //     } => todo!(),
            //     types::MotionCommand::InWalkKick {
            //         head,
            //         kick,
            //         kicking_side,
            //     } => todo!(),
            //     _ => {}
            // }
        }
    }
}
