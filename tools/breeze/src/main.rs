use cgos::board::BoardClass;
use cgos::congatec::Congatec;
use cgos::status::Status;

#[derive(Debug)]
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

    dbg!(sensor_state);

    let number_of_fans = board.get_number_of_fans();
    dbg!(number_of_fans);
    for index in 0..number_of_fans {
        let fan = board.get_fan(index);
        dbg!(fan.current());
        let mut info = fan.info();
        dbg!(info);
        info.out_maximum = 40; // <-- use this for setting the fan speed (unit: percent)
        fan.set_limits(info);
    }

    let board = congatec.get_board_from_name("QA32");
    dbg!(board.name());

    let board = congatec.get_board(BoardClass::CPU, 0);
    dbg!(board.name());

    let board = congatec.get_board(BoardClass::VGA, 0);
    dbg!(board.name());
}
