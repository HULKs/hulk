use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, Instant},
};

use color_eyre::Result;
use procfs::{DiskStat, net::DeviceStatus};
use ros_z::prelude::{Message, Publisher};
use tracing::warn;
use types::system_info::{DiskIo, DiskIoRates, NetworkIo, NetworkIoRates};

struct Sample<T> {
    time: Instant,
    data: T,
}

pub(super) async fn run_disk(publisher: Publisher<BTreeMap<String, DiskIo>>) -> Result<()> {
    poll(
        publisher,
        || {
            procfs::diskstats().map(|disks| {
                disks
                    .into_iter()
                    .map(|disk| (disk.name.clone(), disk))
                    .collect()
            })
        },
        disk_metrics,
    )
    .await
}

pub(super) async fn run_network(publisher: Publisher<BTreeMap<String, NetworkIo>>) -> Result<()> {
    poll(publisher, procfs::net::dev_status, network_metrics).await
}

async fn poll<T: Message + Send + Sync + 'static, S: Send + 'static>(
    publisher: Publisher<T>,
    read: fn() -> procfs::ProcResult<S>,
    metrics: fn(&Sample<S>, &Sample<S>) -> T,
) -> Result<()> {
    let mut timer = crate::sample_timer();
    let mut previous = None;
    loop {
        timer.tick().await;
        if !publisher.has_subscribers() {
            previous = None;
            continue;
        }
        // procfs reads are blocking. Timestamp each sample on the blocking thread.
        match tokio::task::spawn_blocking(move || {
            read().map(|data| Sample {
                time: Instant::now(),
                data,
            })
        })
        .await?
        {
            Ok(current) => {
                let metrics = previous
                    .as_ref()
                    .map(|previous| metrics(&current, previous));
                previous = Some(current);
                if let Some(metrics) = metrics
                    && publisher.has_subscribers()
                {
                    publisher.publish(&metrics).await?;
                }
            }
            Err(error) => {
                warn!(%error, topic = %publisher.entity().topic, "could not read I/O counters");
                previous = None;
            }
        }
    }
}

fn disk_metrics(
    current: &Sample<BTreeMap<String, DiskStat>>,
    previous: &Sample<BTreeMap<String, DiskStat>>,
) -> BTreeMap<String, DiskIo> {
    current
        .data
        .iter()
        .filter_map(|(name, disk)| {
            let old = previous.data.get(name)?;
            if old.major != disk.major || old.minor != disk.minor {
                return None;
            }
            Some((
                name.clone(),
                DiskIo {
                    outstanding_requests: disk.in_progress,
                    rates: disk_rates(disk, old, current.time.duration_since(previous.time))?,
                },
            ))
        })
        .collect()
}

fn network_metrics(
    current: &Sample<HashMap<String, DeviceStatus>>,
    previous: &Sample<HashMap<String, DeviceStatus>>,
) -> BTreeMap<String, NetworkIo> {
    current
        .data
        .iter()
        .filter_map(|(name, device)| {
            Some((
                name.clone(),
                NetworkIo {
                    rates: network_rates(
                        device,
                        previous.data.get(name)?,
                        current.time.duration_since(previous.time),
                    )?,
                },
            ))
        })
        .collect()
}

fn disk_rates(current: &DiskStat, previous: &DiskStat, elapsed: Duration) -> Option<DiskIoRates> {
    if elapsed.is_zero() {
        return None;
    }
    let seconds = elapsed.as_secs_f64();
    let reads = current.reads.checked_sub(previous.reads)?;
    let writes = current.writes.checked_sub(previous.writes)?;
    let read_ms = current.time_reading.checked_sub(previous.time_reading)?;
    let write_ms = current.time_writing.checked_sub(previous.time_writing)?;
    Some(DiskIoRates {
        // Linux diskstats sectors are always 512 bytes, regardless of device sector size.
        read_bytes_per_second: current.sectors_read.checked_sub(previous.sectors_read)? as f64
            * 512.0
            / seconds,
        write_bytes_per_second: current
            .sectors_written
            .checked_sub(previous.sectors_written)? as f64
            * 512.0
            / seconds,
        read_operations_per_second: reads as f64 / seconds,
        write_operations_per_second: writes as f64 / seconds,
        average_read_latency_ms: (reads > 0).then(|| read_ms as f64 / reads as f64),
        average_write_latency_ms: (writes > 0).then(|| write_ms as f64 / writes as f64),
    })
}

fn network_rates(
    current: &DeviceStatus,
    previous: &DeviceStatus,
    elapsed: Duration,
) -> Option<NetworkIoRates> {
    if elapsed.is_zero() {
        return None;
    }
    let seconds = elapsed.as_secs_f64();
    let rate = |current: u64, previous: u64| {
        current
            .checked_sub(previous)
            .map(|delta| delta as f64 / seconds)
    };
    Some(NetworkIoRates {
        receive_bytes_per_second: rate(current.recv_bytes, previous.recv_bytes)?,
        transmit_bytes_per_second: rate(current.sent_bytes, previous.sent_bytes)?,
        receive_errors_per_second: rate(current.recv_errs, previous.recv_errs)?,
        transmit_errors_per_second: rate(current.sent_errs, previous.sent_errs)?,
        receive_drops_per_second: rate(current.recv_drop, previous.recv_drop)?,
        transmit_drops_per_second: rate(current.sent_drop, previous.sent_drop)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk(reads: u64, sectors: u64, milliseconds: u64) -> DiskStat {
        DiskStat::from_line(&format!(
            "259 0 nvme0n1 {reads} 0 {sectors} {milliseconds} 0 0 0 0 3 0 0"
        ))
        .unwrap()
    }

    #[test]
    fn disk_rates_use_elapsed_time_and_fixed_sector_units() {
        let rates = disk_rates(
            &disk(14, 120, 24),
            &disk(10, 100, 12),
            Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(rates.read_bytes_per_second, 5120.0);
        assert_eq!(rates.read_operations_per_second, 2.0);
        assert_eq!(rates.average_read_latency_ms, Some(3.0));
        assert_eq!(rates.average_write_latency_ms, None);
        assert_eq!(rates.write_bytes_per_second, 0.0);
        assert!(disk_rates(&disk(1, 1, 1), &disk(10, 100, 12), Duration::from_secs(1)).is_none());
    }

    #[test]
    fn disk_metrics_skip_new_devices_zero_intervals_and_resets() {
        let time = Instant::now();
        let first = Sample {
            time,
            data: BTreeMap::from([("nvme0n1".into(), disk(10, 100, 12))]),
        };
        assert!(disk_metrics(&first, &first).is_empty());
        let mut previous = Sample {
            time: time - Duration::from_secs(1),
            data: BTreeMap::new(),
        };
        assert!(disk_metrics(&first, &previous).is_empty());
        previous.data = BTreeMap::from([("nvme0n1".into(), disk(1, 1, 1))]);
        let metrics = disk_metrics(&first, &previous);
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics["nvme0n1"].outstanding_requests, 3);
        assert_eq!(metrics["nvme0n1"].rates.read_operations_per_second, 9.0);
        previous.data.get_mut("nvme0n1").unwrap().minor = 1;
        assert!(disk_metrics(&first, &previous).is_empty());
        previous.data = BTreeMap::from([("nvme0n1".into(), disk(20, 200, 24))]);
        assert!(disk_metrics(&first, &previous).is_empty());
    }

    #[test]
    fn network_rates_and_counter_reset() {
        let device = |received, sent| DeviceStatus {
            name: "eth0".into(),
            recv_bytes: received,
            sent_bytes: sent,
            recv_packets: 0,
            recv_errs: 4,
            recv_drop: 2,
            recv_fifo: 0,
            recv_frame: 0,
            recv_compressed: 0,
            recv_multicast: 0,
            sent_packets: 0,
            sent_errs: 2,
            sent_drop: 1,
            sent_fifo: 0,
            sent_colls: 0,
            sent_carrier: 0,
            sent_compressed: 0,
        };
        let rates =
            network_rates(&device(300, 600), &device(100, 200), Duration::from_secs(2)).unwrap();
        assert_eq!(rates.receive_bytes_per_second, 100.0);
        assert_eq!(rates.transmit_bytes_per_second, 200.0);
        assert_eq!(rates.receive_errors_per_second, 0.0);
        assert!(
            network_rates(&device(0, 600), &device(100, 200), Duration::from_secs(1)).is_none()
        );
        assert!(network_rates(&device(300, 600), &device(100, 200), Duration::ZERO).is_none());
        let time = Instant::now();
        let first = Sample {
            time,
            data: HashMap::from([("eth0".into(), device(300, 600))]),
        };
        assert!(network_metrics(&first, &first).is_empty());
        let mut previous = Sample {
            time: time - Duration::from_secs(1),
            data: HashMap::new(),
        };
        assert!(network_metrics(&first, &previous).is_empty());
        previous.data = HashMap::from([("eth0".into(), device(100, 200))]);
        let metrics = network_metrics(&first, &previous);
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics["eth0"].rates.receive_bytes_per_second, 200.0);
        previous.data = HashMap::from([("eth0".into(), device(400, 800))]);
        assert!(network_metrics(&first, &previous).is_empty());
    }
}
