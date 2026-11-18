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
    let mut motion_affinity = CpuSet::new();
    for cpu in motion_cpus {
        motion_affinity.set(cpu)?;
    }
    let (thread_started, mut thread_starts) = unbounded_channel();

    let runtime = Builder::new_multi_thread()
        .worker_threads(worker_count)
        .thread_name("hulk-motion")
        .on_thread_start(move || {
            // Apply the whole set to every async and blocking worker, including
            // replacements created by block_in_place. Linux chooses their CPU.
            let result = sched_setaffinity(Pid::from_raw(0), &motion_affinity)
                .wrap_err("failed to set motion worker CPU affinity");
            let _ = thread_started.send(result);
        })
        .enable_all()
        .build()
        .wrap_err("failed to build motion runtime")?;

    // Reject initial affinity failures before starting nodes. Keep the receiver
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
