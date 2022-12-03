use serde::Serialize;

use crate::service_manager::SystemServices;

#[derive(Clone, Debug, Serialize)]
struct BeaconResponse<'a> {
    hostname: &'a str,
    interface_name: &'a str,
    system_services: SystemServices,
    hulks_os_version: &'a str,
    body_id: &'a str,
    head_id: &'a str,
    battery_charge: f32,
    battery_current: f32,
}

// fn send_beacon(
//     source_address: Ipv4Addr,
//     target_address: Ipv4Addr,
//     port: u16,
//     message: &BeaconResponse<'_>,
// ) -> Result<usize> {
//     let source_address = SocketAddr::from((source_address, port));
//     let socket = UdpSocket::bind(source_address)
//         .with_context(|| format!("Failed to bind UDP socket to {}", source_address))?;
//     let data = serde_json::to_vec(&message)?;
//     debug!("Sending from {}", source_address);
//     socket
//         .send_to(&data, SocketAddrV4::new(target_address, port))
//         .context("Failed to send to UDP socket")
// }
//
// pub fn send_beacon_on_all_interfaces(
//     target_address: Ipv4Addr,
//     port: u16,
//     hostname: &OsString,
//     system_services: SystemServices,
//     hulks_os_version: &String,
//     robot_configuration: RobotConfiguration,
//     battery: Battery,
// ) -> Result<()> {
//     debug!("Send beacon to all interfaces");
//     let active_interfaces = interfaces()
//         .into_iter()
//         .filter(|interface| interface.is_up() && interface.is_multicast());
//
//     let body_id = std::str::from_utf8(&robot_configuration.body_id)?;
//     let head_id = std::str::from_utf8(&robot_configuration.head_id)?;
//
//     for interface in active_interfaces {
//         let message = BeaconResponse {
//             hostname,
//             interface_name: interface.name,
//             system_services: &system_services,
//             hulks_os_version,
//             body_id,
//             head_id,
//             battery_charge: battery.charge,
//             battery_current: battery.current,
//         };
//
//         debug!("Sending {:?}", message);
//
//         for ip in interface.ips {
//             let ip = match ip {
//                 IpNetwork::V4(network) => network.ip(),
//                 IpNetwork::V6(_) => continue,
//             };
//             send_beacon(ip, target_address, port, &message)?;
//         }
//     }
//
//     Ok(())
// }
