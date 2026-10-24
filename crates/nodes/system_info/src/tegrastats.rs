use std::process::Stdio;

use color_eyre::{
    Result,
    eyre::{ensure, eyre},
};
use ros_z::prelude::Publisher;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};
use tracing::warn;
use types::system_info::{CpuInfo, MemoryUsage, Power, Tegrastats, Temperature};

pub(super) async fn run(publisher: Publisher<Tegrastats>, mut command: Command) -> Result<()> {
    command.stdout(Stdio::piped()).kill_on_drop(true);
    let mut timer = crate::sample_timer();
    loop {
        timer.tick().await;
        if !publisher.has_subscribers() {
            continue;
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                warn!(%error, "could not start tegrastats, retrying");
                continue;
            }
        };
        let stdout = child.stdout.take().expect("stdout was configured as piped");
        let mut lines = BufReader::new(stdout).lines();
        loop {
            tokio::select! {
                biased;
                _ = timer.tick() => {
                    if !publisher.has_subscribers() {
                        child.kill().await?;
                        break;
                    }
                }
                line = lines.next_line() => {
                    let line = match line {
                        Ok(Some(line)) => line,
                        result => {
                            warn!(?result, "tegrastats output ended, retrying");
                            break;
                        }
                    };
                    if !publisher.has_subscribers() {
                        child.kill().await?;
                        break;
                    }
                    match parse(&line) {
                        Ok(info) => publisher.publish(&info).await?,
                        Err(error) => warn!(%error, "ignoring malformed tegrastats sample"),
                    }
                }
            }
        }
    }
}

fn parse(line: &str) -> Result<Tegrastats> {
    let mut info = Tegrastats::default();
    let mut fields = line.split_whitespace();
    while let Some(field) = fields.next() {
        match field {
            "RAM" | "SWAP" => {
                let (used, total) = fields
                    .next()
                    .and_then(|value| value.strip_suffix("MB"))
                    .and_then(|value| value.split_once('/'))
                    .ok_or_else(|| eyre!("invalid {field} usage"))?;
                let usage = MemoryUsage {
                    used_mib: used.parse()?,
                    total_mib: total.parse()?,
                };
                ensure!(
                    usage.used_mib <= usage.total_mib,
                    "invalid {field} capacity"
                );
                if field == "RAM" {
                    info.ram = Some(usage);
                } else {
                    info.swap = Some(usage);
                }
            }
            "CPU" => {
                let cores = fields
                    .next()
                    .and_then(|value| value.strip_prefix('['))
                    .and_then(|value| value.strip_suffix(']'))
                    .ok_or_else(|| eyre!("invalid CPU readings"))?;
                for core in cores.split(',') {
                    info.cpus.push(if core == "off" {
                        CpuInfo::default()
                    } else {
                        let (usage, frequency) = core
                            .split_once("%@")
                            .ok_or_else(|| eyre!("invalid CPU core reading"))?;
                        CpuInfo {
                            utilization_percent: Some(percent(usage)?),
                            frequency_mhz: Some(frequency.parse()?),
                        }
                    });
                }
            }
            "GR3D_FREQ" => {
                let value = fields.next().ok_or_else(|| eyre!("missing GPU reading"))?;
                if let Some((usage, _)) = value.split_once('%') {
                    info.gpu_utilization_percent = Some(percent(usage)?);
                }
            }
            _ if field.ends_with('C') && field.contains('@') => {
                let (name, value) = field[..field.len() - 1].split_once('@').unwrap();
                let celsius: f32 = value.parse()?;
                ensure!(celsius.is_finite(), "invalid temperature");
                info.temperatures.push(Temperature {
                    name: name.to_owned(),
                    celsius,
                });
            }
            _ if field.starts_with("VDD_") => {
                let (current, average) = fields
                    .next()
                    .and_then(|value| value.split_once('/'))
                    .ok_or_else(|| eyre!("invalid power reading"))?;
                let milliwatts = |value: &str| -> Result<u32> {
                    Ok(value
                        .strip_suffix("mW")
                        .ok_or_else(|| eyre!("invalid power unit"))?
                        .parse()?)
                };
                info.power.push(Power {
                    name: field.to_owned(),
                    current_milliwatts: milliwatts(current)?,
                    average_milliwatts: milliwatts(average)?,
                });
            }
            _ => {}
        }
    }
    ensure!(
        info.ram.is_some() && !info.cpus.is_empty(),
        "missing RAM or CPU readings"
    );
    Ok(info)
}

fn percent(value: &str) -> Result<u8> {
    let value = value.parse()?;
    ensure!(value <= 100, "utilization exceeds 100 percent");
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "10-24-2026 19:08:06 RAM 4996/7621MB (lfb 2x4MB) SWAP 104/3810MB (cached 2MB) CPU [84%@1958,84%@1958,89%@1958,89%@1958,25%@1958,44%@1958] GR3D_FREQ 75% cv0@93.031C cpu@98.812C iwlwifi_1@92C soc2@92.593C soc0@93.562C cv1@92.718C gpu@97.968C tj@99C soc1@92.437C cv2@89.281C VDD_IN 27713mW/27713mW VDD_CPU_GPU_CV 15271mW/15271mW VDD_SOC 6065mW/6065mW";

    #[test]
    fn parses_orin_sample() {
        let info = parse(SAMPLE).unwrap();
        assert_eq!(info.cpus.len(), 6);
        assert_eq!(info.cpus[0].utilization_percent, Some(84));
        assert_eq!(info.cpus[0].frequency_mhz, Some(1958));
        assert_eq!(info.gpu_utilization_percent, Some(75));
        assert_eq!(info.ram.unwrap().used_mib, 4996);
        assert_eq!(info.swap.unwrap().total_mib, 3810);
        assert_eq!(info.temperatures.len(), 10);
        assert_eq!(info.temperatures[7].name, "tj");
        assert_eq!(info.temperatures[7].celsius, 99.0);
        assert_eq!(info.power.len(), 3);
        assert_eq!(info.power[0].current_milliwatts, 27713);
    }

    #[test]
    fn missing_gpu_and_offline_cpu_are_not_zero() {
        let info = parse("RAM 0/1MB CPU [off,0%@100]").unwrap();
        assert_eq!(info.cpus[0], CpuInfo::default());
        assert_eq!(info.cpus[1].utilization_percent, Some(0));
        assert_eq!(info.gpu_utilization_percent, None);
        assert_eq!(info.swap, None);
        assert!(info.power.is_empty());
        assert_eq!(
            parse("RAM 0/1MB CPU [off] GR3D_FREQ 75%@[100,100]")
                .unwrap()
                .gpu_utilization_percent,
            Some(75)
        );
    }

    #[test]
    fn rejects_malformed_readings() {
        for line in [
            "",
            "RAM 2/1MB CPU [off]",
            "RAM 0/1MB CPU [101%@100]",
            "RAM 0/1MB CPU [off] gpu@NaNC",
            "RAM 0/1MB CPU [off] VDD_IN invalid",
        ] {
            assert!(parse(line).is_err(), "{line}");
        }
    }
}
