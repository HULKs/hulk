use std::cmp;

use cgos::board::BoardClass;
use cgos::congatec::Congatec;
use cgos::status::Status;

static FAN_MAX_SPEED: u8 = 100;
static FAN_MIN_SPEED: u8 = 10;
static INTERPOLATION_X0: u8 = 0;
static INTERPOLATION_Y0: u8 = 0;
static INTERPOLATION_X1: u8 = 100;
static INTERPOLATION_Y1: u8 = 100;

#[derive(Debug, Clone)]
enum SensorState {
    Uninitialized,
    Broken,
    Valid(f32),
}

fn main() {
    let congatec = Congatec::new();
    let board = congatec.get_board(BoardClass::ALL, 0);

    let number_of_temperatures = board.get_number_of_temperatures();
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

    let fan_speed = get_interpolated_fan_speed(sensor_state.clone());
    dbg!(sensor_state);
    dbg!(fan_speed);

    let number_of_fans = board.get_number_of_fans();
    for index in 0..number_of_fans {
        let fan = board.get_fan(index);
        let mut info = fan.info();
        info.out_maximum = fan_speed as i32;
        fan.set_limits(info);
    }
}

fn get_interpolated_fan_speed(sensor_state: SensorState) -> u8 {
    if let SensorState::Valid(temperature) = sensor_state {
        let fan_value = interpolate(
            temperature as u8,
            INTERPOLATION_X0,
            INTERPOLATION_Y0,
            INTERPOLATION_X1,
            INTERPOLATION_Y1,
        );
        cmp::max(FAN_MIN_SPEED, fan_value)
    } else {
        // Something is wrong with the temperature sensor.
        // Lets crank the fans up.
        FAN_MAX_SPEED
    }
}

fn interpolate(x: u8, x0: u8, y0: u8, x1: u8, y1: u8) -> u8 {
    (y0 * (x1 - x) + y1 * (x - x0)) / (x1 - x0)
}
