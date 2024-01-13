use std::{thread::sleep, time::Duration};

use cgos::board::BoardClass;
use cgos::congatec::Congatec;
use cgos::status::Status;

const FAN_MAX_SPEED: f32 = 100.0;
const FAN_MIN_SPEED: f32 = 40.0; // should be at least ~38 to allow spin up from zero
const INTERPOLATION_X0: f32 = 50.0;
const INTERPOLATION_Y0: f32 = 50.0;
const INTERPOLATION_X1: f32 = 70.0;
const INTERPOLATION_Y1: f32 = 100.0;
const SLEEP_DURATION: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
enum SensorState {
    Uninitialized,
    Broken,
    Valid(f32),
}

fn main() {
    let congatec = Congatec::new();
    let board = congatec.get_board(BoardClass::ALL, 0);
    let number_of_temperatures = board.get_number_of_temperatures();

    loop {
        let sensor_state =
            (0..number_of_temperatures).fold(SensorState::Uninitialized, |sensor_state, index| {
                let sensor = board.get_temperature(index);
                let (current_temperature, current_status) = sensor.current();

                match (sensor_state, current_status == Status::ACTIVE) {
                    (SensorState::Uninitialized, true) => SensorState::Valid(current_temperature),
                    (SensorState::Uninitialized, false)
                    | (SensorState::Broken, _)
                    | (SensorState::Valid(_), false) => SensorState::Broken,
                    (SensorState::Valid(temperature), true) => {
                        SensorState::Valid(temperature.max(current_temperature))
                    }
                }
            });

        let fan_speed = get_interpolated_fan_speed(sensor_state);
        let number_of_fans = board.get_number_of_fans();
        for index in 0..number_of_fans {
            let fan = board.get_fan(index);
            let mut info = fan.info();
            info.out_maximum = fan_speed as i32;
            fan.set_limits(info);
        }

        sleep(SLEEP_DURATION);
    }
}

fn get_interpolated_fan_speed(sensor_state: SensorState) -> f32 {
    if let SensorState::Valid(temperature) = sensor_state {
        let fan_value = interpolate(
            temperature,
            INTERPOLATION_X0,
            INTERPOLATION_Y0,
            INTERPOLATION_X1,
            INTERPOLATION_Y1,
        );
        fan_value.clamp(FAN_MIN_SPEED, FAN_MAX_SPEED)
    } else {
        // Something is wrong with the temperature sensor.
        // Lets crank the fans up.
        FAN_MAX_SPEED
    }
}

fn interpolate(x: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    (y0 * (x1 - x) + y1 * (x - x0)) / (x1 - x0)
}
