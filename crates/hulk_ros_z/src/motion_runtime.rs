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

pub struct MotionRuntimes {
    pub runtimes: Vec<Runtime>,
    pub thread_starts: UnboundedReceiver<Result<()>>,
}

/// Call before creating the general runtime or transport threads. The caller and
/// its future children exclude the motion CPUs.
#[cfg(target_os = "linux")]
pub fn build(cpus: &[usize]) -> Result<MotionRuntimes> {
    let allowed_cpus =
        sched_getaffinity(Pid::from_raw(0)).wrap_err("failed to read startup CPU affinity")?;
    let (motion_cpus, general_cpus) = partition_cpus(cpus, allowed_cpus)?;
    let worker_count = motion_cpus.len();
    let (thread_started, mut thread_starts) = unbounded_channel();
    let mut runtimes = Vec::with_capacity(worker_count);
    for cpu in motion_cpus {
        let thread_started = thread_started.clone();
        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name(format!("hulk-motion-{cpu}"))
            .on_thread_start(move || {
                // block_in_place can transfer the scheduler onto a blocking-pool
                // thread. Every thread in this runtime must use the same CPU.
                let _ = thread_started.send(pin_current_thread(cpu));
            })
            .enable_all()
            .build()
            .wrap_err_with(|| format!("failed to build motion runtime for CPU {cpu}"))?;
        runtimes.push(runtime);
    }

    // Reject initial pinning failures before starting nodes. Keep the receiver
    // so the stack can also monitor failures from later blocking workers.
    for _ in 0..worker_count {
        thread_starts
            .blocking_recv()
            .ok_or_else(|| eyre!("motion runtime stopped during startup"))??;
    }

    sched_setaffinity(Pid::from_raw(0), &general_cpus)
        .wrap_err("failed to restrict the main thread to the general CPU set")?;
    Ok(MotionRuntimes {
        runtimes,
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
