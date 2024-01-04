use cgos::congatec::Congatec;
use cgos::board::BoardClass;

fn main() {
    let congatec = Congatec::new();
    dbg!(congatec.get_number_of_boards(BoardClass::ALL));
    dbg!(congatec.get_number_of_boards(BoardClass::CPU));
    dbg!(congatec.get_number_of_boards(BoardClass::VGA));
    dbg!(congatec.get_number_of_boards(BoardClass::IO));

     let board = congatec.get_board(BoardClass::ALL, 0);
     dbg!(board.name());
     dbg!(board.info());
     dbg!(board.boot_count());
     dbg!(board.running_time());
     let number_of_temperatures = board.get_number_of_temperatures();
     dbg!(number_of_temperatures);
     for index in 0..number_of_temperatures {
         let temperature = board.get_temperature(index);
         dbg!(temperature.info());
         dbg!(temperature.current()); // <-- use this for getting the temperature (unit: Celcius)
     }

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