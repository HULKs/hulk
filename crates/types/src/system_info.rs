use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct Tegrastats {
    /// CPU index is the position in this vector.
    pub cpus: Vec<CpuInfo>,
    pub gpu_utilization_percent: Option<u8>,
    pub ram: Option<MemoryUsage>,
    pub swap: Option<MemoryUsage>,
    pub temperatures: Vec<Temperature>,
    pub power: Vec<Power>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct CpuInfo {
    pub utilization_percent: Option<u8>,
    pub frequency_mhz: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct MemoryUsage {
    pub used_mib: u64,
    pub total_mib: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct Temperature {
    pub name: String,
    pub celsius: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct Power {
    pub name: String,
    pub current_milliwatts: u32,
    pub average_milliwatts: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct DiskIo {
    pub outstanding_requests: u64,
    pub rates: DiskIoRates,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct DiskIoRates {
    pub read_bytes_per_second: f64,
    pub write_bytes_per_second: f64,
    pub read_operations_per_second: f64,
    pub write_operations_per_second: f64,
    /// Absent when no corresponding operations completed during the interval.
    pub average_read_latency_ms: Option<f64>,
    pub average_write_latency_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct NetworkIo {
    pub rates: NetworkIoRates,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct NetworkIoRates {
    pub receive_bytes_per_second: f64,
    pub transmit_bytes_per_second: f64,
    pub receive_errors_per_second: f64,
    pub transmit_errors_per_second: f64,
    pub receive_drops_per_second: f64,
    pub transmit_drops_per_second: f64,
}
