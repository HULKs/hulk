use std::{
    future::Future,
    io::{self, Write},
    num::NonZeroUsize,
    pin::Pin,
    time::{Duration, Instant},
};

use color_eyre::eyre::{Result, WrapErr, eyre};
use ros_z::{attachment::Attachment, pubsub::RawSubscriber};
use tokio::time::{MissedTickBehavior, Sleep};

use crate::{
    app::AppContext,
    cli::HzLimit,
    model::hz::{HzEstimator, HzReport},
    render::{OutputMode, json, text},
};

const TYPE_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);
const REPORT_PERIOD: Duration = Duration::from_secs(1);

#[derive(Default)]
struct HzReceiveState {
    warned_invalid_attachment: bool,
}

fn decode_attachment_for_hz(
    attachment: Option<&zenoh::bytes::ZBytes>,
    state: &mut HzReceiveState,
) -> Option<Attachment> {
    let attachment = attachment?;

    match Attachment::try_from(attachment) {
        Ok(attachment) => Some(attachment),
        Err(error) => {
            if !state.warned_invalid_attachment {
                let _ = writeln!(
                    io::stderr(),
                    "warning: failed to decode ros-z attachment for hz source stats: {error}"
                );
                state.warned_invalid_attachment = true;
            }
            None
        }
    }
}

pub async fn run(
    app: &AppContext,
    output_mode: OutputMode,
    topic: &str,
    window: NonZeroUsize,
    limit: HzLimit,
) -> Result<()> {
    let mut subscriber = app
        .create_raw_subscriber_builder(topic, TYPE_DISCOVERY_TIMEOUT)
        .build()
        .await
        .wrap_err_with(|| format!("failed to subscribe to {topic}"))?;
    let mut estimator = HzEstimator::new(topic.to_string(), window);
    let mut receive_state = HzReceiveState::default();

    match limit {
        HzLimit::Count(count) => {
            run_count(
                &mut subscriber,
                &mut estimator,
                &mut receive_state,
                output_mode,
                count,
            )
            .await
        }
        HzLimit::Duration(duration) => {
            run_duration(
                &mut subscriber,
                &mut estimator,
                &mut receive_state,
                output_mode,
                duration,
            )
            .await
        }
        HzLimit::Continuous => {
            run_continuous(
                &mut subscriber,
                &mut estimator,
                &mut receive_state,
                output_mode,
            )
            .await
        }
    }
}

async fn run_count(
    subscriber: &mut RawSubscriber,
    estimator: &mut HzEstimator,
    state: &mut HzReceiveState,
    output_mode: OutputMode,
    count: NonZeroUsize,
) -> Result<()> {
    for _ in 0..count.get() {
        receive_one(subscriber, estimator, state).await?;
    }
    print_report(&estimator.report(), output_mode)
}

async fn run_duration(
    subscriber: &mut RawSubscriber,
    estimator: &mut HzEstimator,
    state: &mut HzReceiveState,
    output_mode: OutputMode,
    duration: Duration,
) -> Result<()> {
    let deadline = tokio::time::Instant::now()
        .checked_add(duration)
        .ok_or_else(|| eyre!("duration is too large for monotonic clock deadline"))?;
    let mut reports = tokio::time::interval(REPORT_PERIOD);
    reports.set_missed_tick_behavior(MissedTickBehavior::Delay);
    reports.tick().await;
    let deadline_sleep = tokio::time::sleep_until(deadline);
    tokio::pin!(deadline_sleep);

    loop {
        match select_duration_event(
            receive_one(subscriber, estimator, state),
            tokio::signal::ctrl_c(),
            &mut reports,
            deadline_sleep.as_mut(),
        )
        .await
        {
            DurationEvent::Deadline => {
                print_report(&estimator.report(), output_mode)?;
                return Ok(());
            }
            DurationEvent::Receive(result) => result?,
            DurationEvent::Report => {
                if should_print_periodic_report(tokio::time::Instant::now(), deadline) {
                    print_report(&estimator.report(), output_mode)?;
                }
            }
            DurationEvent::Interrupted(signal) => {
                signal.wrap_err("failed to listen for Ctrl-C")?;
                return Ok(());
            }
        }
    }
}

enum DurationEvent<Receive> {
    Deadline,
    Receive(Receive),
    Report,
    Interrupted(std::io::Result<()>),
}

async fn select_duration_event<Receive, Interrupt>(
    receive: Receive,
    interrupt: Interrupt,
    reports: &mut tokio::time::Interval,
    mut deadline_sleep: Pin<&mut Sleep>,
) -> DurationEvent<Receive::Output>
where
    Receive: Future,
    Interrupt: Future<Output = std::io::Result<()>>,
{
    if deadline_sleep.deadline() <= tokio::time::Instant::now() {
        return DurationEvent::Deadline;
    }

    tokio::select! {
        biased;
        _ = deadline_sleep.as_mut() => DurationEvent::Deadline,
        _ = reports.tick() => DurationEvent::Report,
        signal = interrupt => DurationEvent::Interrupted(signal),
        result = receive => DurationEvent::Receive(result),
    }
}

fn should_print_periodic_report(now: tokio::time::Instant, deadline: tokio::time::Instant) -> bool {
    now < deadline
}

async fn run_continuous(
    subscriber: &mut RawSubscriber,
    estimator: &mut HzEstimator,
    state: &mut HzReceiveState,
    output_mode: OutputMode,
) -> Result<()> {
    let mut reports = tokio::time::interval(REPORT_PERIOD);
    reports.set_missed_tick_behavior(MissedTickBehavior::Delay);
    reports.tick().await;

    loop {
        tokio::select! {
            signal = tokio::signal::ctrl_c() => {
                signal.wrap_err("failed to listen for Ctrl-C")?;
                return Ok(());
            }
            result = receive_one(subscriber, estimator, state) => result?,
            _ = reports.tick() => print_report(&estimator.report(), output_mode)?,
        }
    }
}

async fn receive_one(
    subscriber: &mut RawSubscriber,
    estimator: &mut HzEstimator,
    state: &mut HzReceiveState,
) -> Result<()> {
    let sample = subscriber.recv().await?;
    estimator.observe_receive(Instant::now());

    if let Some(attachment) = decode_attachment_for_hz(sample.attachment(), state) {
        estimator.observe_source(attachment.source_global_id, attachment.source_time());
    }

    Ok(())
}

fn print_report(report: &HzReport, output_mode: OutputMode) -> Result<()> {
    match output_mode {
        OutputMode::Json => json::print_line(report),
        OutputMode::Text => {
            text::print_hz_report(report);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_periodic_report_is_not_due_at_deadline() {
        let started = tokio::time::Instant::now();
        let deadline = started + REPORT_PERIOD;

        assert!(should_print_periodic_report(started, deadline));
        assert!(!should_print_periodic_report(deadline, deadline));
        assert!(!should_print_periodic_report(
            deadline + Duration::from_nanos(1),
            deadline
        ));
    }

    #[tokio::test]
    async fn duration_deadline_wins_over_ready_receive() {
        let mut reports = tokio::time::interval(REPORT_PERIOD);
        reports.tick().await;
        let deadline = tokio::time::Instant::now();

        let deadline_sleep = tokio::time::sleep_until(deadline);
        tokio::pin!(deadline_sleep);

        let event = select_duration_event(
            std::future::ready("received"),
            std::future::pending::<std::io::Result<()>>(),
            &mut reports,
            deadline_sleep.as_mut(),
        )
        .await;

        assert!(matches!(event, DurationEvent::Deadline));
    }

    #[tokio::test]
    async fn duration_report_wins_over_ready_receive_when_due() {
        let mut reports = tokio::time::interval(Duration::from_millis(1));
        reports.tick().await;
        tokio::time::sleep(Duration::from_millis(2)).await;
        let deadline = tokio::time::Instant::now() + REPORT_PERIOD;

        let deadline_sleep = tokio::time::sleep_until(deadline);
        tokio::pin!(deadline_sleep);

        let event = select_duration_event(
            std::future::ready("received"),
            std::future::pending::<std::io::Result<()>>(),
            &mut reports,
            deadline_sleep.as_mut(),
        )
        .await;

        assert!(matches!(event, DurationEvent::Report));
    }

    #[tokio::test]
    async fn duration_interrupt_wins_over_ready_receive_when_report_not_due() {
        let mut reports = tokio::time::interval(REPORT_PERIOD);
        reports.tick().await;
        let deadline = tokio::time::Instant::now() + REPORT_PERIOD;

        let deadline_sleep = tokio::time::sleep_until(deadline);
        tokio::pin!(deadline_sleep);

        let event = select_duration_event(
            std::future::ready("received"),
            std::future::ready(Ok(())),
            &mut reports,
            deadline_sleep.as_mut(),
        )
        .await;

        assert!(matches!(event, DurationEvent::Interrupted(Ok(()))));
    }

    #[test]
    fn invalid_attachment_sets_warning_state_once() {
        let malformed = zenoh::bytes::ZBytes::from(vec![0x01]);
        let mut state = HzReceiveState::default();

        assert!(decode_attachment_for_hz(Some(&malformed), &mut state).is_none());
        assert!(state.warned_invalid_attachment);

        assert!(decode_attachment_for_hz(Some(&malformed), &mut state).is_none());
        assert!(state.warned_invalid_attachment);
    }
}
