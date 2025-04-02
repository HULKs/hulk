use std::io::Cursor;
use std::thread::sleep;
use std::time::Duration;

use color_eyre::eyre::{eyre, Result, WrapErr};
use rodio::{source::Buffered, Decoder, OutputStream, Sink, Source};
use zbus::{proxy, Connection, zvariant::Optional};

use hula_types::Battery;

const AUDIO_FILE: &[u8] = include_bytes!("../sound/water-drop.mp3");

#[proxy(
    default_service = "org.hulks.hula",
    interface = "org.hulks.hula",
    default_path = "/org/hulks/HuLA"
)]
trait BatteryInfo {
    fn battery(&self) -> zbus::Result<Optional<Battery>>;
}

struct BatteryInfo {
    proxy: BatteryInfoProxy<'static>,
}

impl BatteryInfo {
    pub async fn initialize() -> Result<Self> {
        let connection = Connection::system().await?;
        let proxy = BatteryInfoProxy::new(&connection)
            .await
            .wrap_err("failed to connect to dbus proxy")?;

        Ok(Self { proxy })
    }

    async fn battery(&self) -> Option<Battery> {
        self.proxy.battery().await.ok().and_then(Option::from)
    }
}

async fn get_battery_info() -> Result<Battery> {
    let battery_info = BatteryInfo::initialize().await?;
    let battery = battery_info
        .battery()
        .await
        .ok_or(eyre!("failed to get battery info"))?;
    Ok(battery)
}

struct AudioPlayer {
    sink: Sink,
    decoder: Buffered<Decoder<Cursor<&'static [u8]>>>,
    _stream: OutputStream,
}

impl AudioPlayer {
    pub fn new(audio_data: &'static [u8]) -> Self {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        let decoder = Decoder::new(Cursor::new(audio_data)).unwrap().buffered();
        Self {
            sink,
            decoder,
            _stream,
        }
    }

    pub fn play(&self) {
        self.sink.append(self.decoder.clone());
        self.sink.sleep_until_end();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let audio_player = AudioPlayer::new(AUDIO_FILE);
    loop {
        let battery = get_battery_info().await?;
        println!("Battery: {:?}", battery);

        let mut time_to_sleep = Duration::from_secs(60);

        let battery_is_low = battery.charge < 0.20;
        let battery_is_charging = battery.current > 0.0;

        if  battery_is_low && !battery_is_charging {
            audio_player.play();
            time_to_sleep = Duration::from_secs((battery.charge * 100.0) as u64);
        }
        
        sleep(time_to_sleep);
    }
}
