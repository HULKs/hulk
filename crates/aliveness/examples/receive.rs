use std::{
    collections::BTreeMap,
    ffi::OsString,
    net::{IpAddr, Ipv4Addr},
    time::{Duration, Instant},
};

use aliveness::{listen_for_beacons, AlivenessMessage, SystemServices};
use color_eyre::eyre::Result;
use colored::Colorize;
use tokio::{
    select,
    time::{interval, MissedTickBehavior},
};

#[derive(Debug)]
struct NaoState {
    hostname: OsString,
    interface_name: String,
    system_services: SystemServices,
    hulks_os_version: String,
    body_id: String,
    head_id: String,
    battery_charge: f32,
    battery_current: f32,
    last_seen: Instant,
}

impl From<AlivenessMessage> for NaoState {
    fn from(message: AlivenessMessage) -> Self {
        Self {
            hostname: message.hostname,
            interface_name: message.interface_name,
            system_services: message.system_services,
            hulks_os_version: message.hulks_os_version,
            body_id: message.body_id,
            head_id: message.head_id,
            battery_charge: message.battery_charge,
            battery_current: message.battery_current,
            last_seen: Instant::now(),
        }
    }
}

impl NaoState {
    fn update(&mut self, message: AlivenessMessage) {
        self.hostname = message.hostname;
        self.interface_name = message.interface_name;
        self.system_services = message.system_services;
        self.hulks_os_version = message.hulks_os_version;
        self.body_id = message.body_id;
        self.head_id = message.head_id;
        self.battery_charge = message.battery_charge;
        self.battery_current = message.battery_current;
        self.last_seen = Instant::now();
    }
}

fn handle_message(
    state_map: &mut BTreeMap<IpAddr, NaoState>,
    nao_address: IpAddr,
    message: AlivenessMessage,
) {
    match state_map.get_mut(&nao_address) {
        Some(entry) => {
            entry.update(message);
        }
        None => {
            state_map.insert(nao_address, NaoState::from(message));
        }
    };
}

fn print_state(state_map: &BTreeMap<IpAddr, NaoState>) {
    print!("\u{1B}[2J");
    print!("\u{1B}[2J\u{1B}[1;1H");
    for (ip, state) in state_map {
        let seconds_since_last_seen = state.last_seen.elapsed().as_secs_f32();
        let formatted_seconds = format!("{seconds_since_last_seen:.2}");
        let elapsed = match seconds_since_last_seen {
            t if t < 1.0 => formatted_seconds.green(),
            t if t < 3.0 => formatted_seconds.yellow(),
            _ => formatted_seconds.red(),
        };
        println!("{ip}: {}s - {}", elapsed, state.head_id);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Start");
    let local_address = Ipv4Addr::new(10, 1, 24, 118);
    let mut receiver = listen_for_beacons(local_address).await?;
    let mut naos = BTreeMap::new();
    let mut interval = interval(Duration::from_millis(500));
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        select! {
             response = receiver.recv() => {
                match response {
                    Some((nao_address, message)) => handle_message(&mut naos,nao_address, message),
                    None => break,
                }
            }
            _ = interval.tick() => {
                print_state(&naos);
            }
        }
    }
    Ok(())
}
