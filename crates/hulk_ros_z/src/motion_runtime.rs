use std::sync::atomic::{AtomicUsize, Ordering};

use color_eyre::{
    Result,
    eyre::{WrapErr, ensure, eyre},
};
use nix::{
    sched::{CpuSet, sched_getaffinity, sched_setaffinity},
    unistd::Pid,
};
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc::{UnboundedReceiver, unbounded_channel},
};

pub struct MotionRuntime {
    pub runtime: Runtime,
    pub thread_starts: UnboundedReceiver<Result<()>>,
}

/// Call before creating the general runtime or transport threads. The caller and
/// its future children exclude the motion CPUs.
#[cfg(target_os = "linux")]
pub fn build(cpus: &[usize]) -> Result<MotionRuntime> {
    let allowed_cpus =
        sched_getaffinity(Pid::from_raw(0)).wrap_err("failed to read startup CPU affinity")?;
    let (motion_cpus, general_cpus) = partition_cpus(cpus, allowed_cpus)?;
    let worker_count = motion_cpus.len();
    let next_cpu = AtomicUsize::new(0);
    let (thread_started, mut thread_starts) = unbounded_channel();

    let runtime = Builder::new_multi_thread()
        .worker_threads(worker_count)
        .thread_name("hulk-motion")
        .on_thread_start(move || {
            // Isolated CPUs do not load-balance. Pin each async or blocking
            // worker individually, before it can execute any application code.
            let index = next_cpu.fetch_add(1, Ordering::Relaxed) % worker_count;
            let _ = thread_started.send(pin_current_thread(motion_cpus[index]));
        })
        .enable_all()
        .build()
        .wrap_err("failed to build motion runtime")?;

    // Reject initial pinning failures before starting nodes. Keep the receiver
    // so the stack can also monitor failures from later blocking workers.
    for _ in 0..worker_count {
        thread_starts
            .blocking_recv()
            .ok_or_else(|| eyre!("motion runtime stopped during startup"))??;
    }

    sched_setaffinity(Pid::from_raw(0), &general_cpus)
        .wrap_err("failed to restrict the main thread to the general CPU set")?;
    Ok(MotionRuntime {
        runtime,
        thread_starts,
    })
}

fn partition_cpus(cpus: &[usize], mut general_cpus: CpuSet) -> Result<(Vec<usize>, CpuSet)> {
    ensure!(!cpus.is_empty(), "motion CPU list must not be empty");
    let mut cpus = cpus.to_vec();
    cpus.sort_unstable();
    cpus.dedup();
    for &cpu in &cpus {
        ensure!(
            general_cpus.is_set(cpu).unwrap_or(false),
            "motion CPU {cpu} is outside the allowed CPU set"
        );
        general_cpus.unset(cpu)?;
    }
    ensure!(
        general_cpus != CpuSet::new(),
        "motion isolation requires at least one other allowed CPU"
    );

    Ok((cpus, general_cpus))
}

fn pin_current_thread(cpu: usize) -> Result<()> {
    let mut affinity = CpuSet::new();
    affinity.set(cpu)?;
    sched_setaffinity(Pid::from_raw(0), &affinity)
        .wrap_err_with(|| format!("failed to pin motion worker to CPU {cpu}"))
}
