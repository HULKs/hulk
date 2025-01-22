use color_eyre::eyre::{eyre, Result, WrapErr};
use zbus::{proxy, zvariant::Optional};

use hula_types::Battery;

use rodio::{source::Buffered, Decoder, OutputStream, Sink, Source};
use std::io::Cursor;

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
        let connection = zbus::Connection::system().await?;
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

        let mut time_to_sleep = 60;
        if battery.charge < 0.20 {
            println!("Battery low, playing sound");
            audio_player.play();
            time_to_sleep = (battery.charge * 50.0) as u64;
        }
        std::thread::sleep(std::time::Duration::from_secs(time_to_sleep));
    }
}
