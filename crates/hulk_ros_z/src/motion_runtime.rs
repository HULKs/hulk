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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::BTreeSet,
        sync::{Arc, Barrier, mpsc},
        time::Duration,
    };

    struct RestoreAffinity(CpuSet);

    impl Drop for RestoreAffinity {
        fn drop(&mut self) {
            sched_setaffinity(Pid::from_raw(0), &self.0).unwrap();
        }
    }

    fn current_cpus() -> Vec<usize> {
        let mask = sched_getaffinity(Pid::from_raw(0)).unwrap();
        (0..CpuSet::count())
            .filter(|&cpu| mask.is_set(cpu).unwrap_or(false))
            .collect()
    }

    #[test]
    fn blocking_handoff_preserves_both_motion_cpus() {
        let original = sched_getaffinity(Pid::from_raw(0)).unwrap();
        let _restore = RestoreAffinity(original);
        let allowed = current_cpus();
        if allowed.len() < 3 {
            eprintln!("requires two motion CPUs and one general CPU");
            return;
        }
        let cpus = &allowed[..2];
        let motion = build(cpus).unwrap();
        let handles: Vec<_> = motion
            .runtimes
            .iter()
            .map(|runtime| runtime.handle().clone())
            .collect();
        assert!(current_cpus().iter().all(|cpu| !cpus.contains(cpu)));
        let timeout = Duration::from_secs(3);

        // Occupy the first blocking-pool thread, as inference initialization can.
        let (prime_started, prime_ready) = mpsc::channel();
        let (prime_release, prime_wait) = mpsc::channel::<()>();
        handles[0].spawn_blocking(move || {
            prime_started.send(current_cpus()).unwrap();
            let _ = prime_wait.recv_timeout(timeout);
        });
        assert_eq!(prime_ready.recv_timeout(timeout).unwrap(), [cpus[0]]);

        // Start one task on each scheduler worker, then force a handoff from the first CPU.
        let start = Arc::new(Barrier::new(2));
        let (entered, blocking) = mpsc::channel();
        let mut releases = vec![];
        for index in 0..2 {
            let start = start.clone();
            let entered = entered.clone();
            let target = cpus[0];
            let (release, wait) = mpsc::channel::<()>();
            releases.push(release);
            handles[index % handles.len()].spawn(async move {
                start.wait();
                if current_cpus() == [target] {
                    tokio::task::block_in_place(|| {
                        entered.send(()).unwrap();
                        let _ = wait.recv_timeout(timeout);
                    });
                }
            });
        }
        blocking.recv_timeout(timeout).unwrap();

        // Hold the first probe until the other worker runs the second probe.
        // This measures two simultaneously usable workers, not creation counters.
        let (observed, observations) = mpsc::channel();
        let mut probe_releases = vec![];
        for index in 0..2 {
            let observed = observed.clone();
            let (release, wait) = mpsc::channel::<()>();
            probe_releases.push(release);
            handles[index % handles.len()].spawn(async move {
                observed.send(current_cpus()).unwrap();
                let _ = wait.recv_timeout(timeout);
            });
        }
        let first = observations.recv_timeout(timeout).unwrap();
        let second = observations.recv_timeout(timeout).unwrap();
        drop(probe_releases);
        drop(releases);
        drop(prime_release);
        for runtime in motion.runtimes {
            runtime.shutdown_timeout(timeout);
        }
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        let observed: BTreeSet<_> = first.into_iter().chain(second).collect();
        assert_eq!(
            observed,
            cpus.iter().copied().collect(),
            "a blocking handoff must not move both scheduler workers onto one CPU"
        );
    }

    #[test]
    fn nested_tasks_and_inference_threads_keep_the_node_cpu() {
        let original = sched_getaffinity(Pid::from_raw(0)).unwrap();
        let _restore = RestoreAffinity(original);
        let allowed = current_cpus();
        if allowed.len() < 3 {
            eprintln!("requires two motion CPUs and one general CPU");
            return;
        }
        let cpus = &allowed[..2];
        let motion = build(cpus).unwrap();
        let mut nodes = Vec::new();
        for (runtime, &cpu) in motion.runtimes.iter().zip(cpus) {
            nodes.push(runtime.spawn(async move {
                for _ in 0..4 {
                    assert_eq!(current_cpus(), [cpu]);
                    tokio::task::yield_now().await;
                    assert_eq!(tokio::spawn(async { current_cpus() }).await.unwrap(), [cpu]);
                    assert_eq!(
                        tokio::task::spawn_blocking(current_cpus).await.unwrap(),
                        [cpu]
                    );
                    assert_eq!(tokio::task::block_in_place(current_cpus), [cpu]);
                    assert_eq!(std::thread::spawn(current_cpus).join().unwrap(), [cpu]);
                    assert_eq!(current_cpus(), [cpu]);
                }
            }));
        }
        motion.runtimes[0].block_on(async {
            for node in nodes {
                tokio::time::timeout(Duration::from_secs(3), node)
                    .await
                    .unwrap()
                    .unwrap();
            }
        });
        for runtime in motion.runtimes {
            runtime.shutdown_timeout(Duration::from_secs(1));
        }
    }
}
