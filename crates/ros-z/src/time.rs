use std::{
    fmt,
    future::Future,
    ops::{Add, Sub},
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

use crate::{Message, SerdeCdrCodec};
use ros_z_schema::TypeName;

/// A clock-relative instant used throughout ros-z.
///
/// `Time` is intentionally generic: it represents an instant on some clock's
/// timeline and only becomes wallclock time when interpreted through a
/// wallclock-backed [`Clock`] or converted with [`Time::from_wallclock`] /
/// [`Time::to_wallclock`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Time {
    since_origin: Duration,
}

impl Message for Time {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "ros_z::Time"
    }

    fn schema() -> crate::dynamic::Schema {
        std::sync::Arc::new(crate::dynamic::TypeShape::Struct {
            name: TypeName::new("ros_z::Time").expect("valid type name"),
            fields: vec![crate::dynamic::RuntimeFieldSchema::new(
                "duration",
                duration_schema(),
            )],
        })
    }
}

fn duration_schema() -> crate::dynamic::Schema {
    std::sync::Arc::new(crate::dynamic::TypeShape::Struct {
        name: TypeName::new("builtin_interfaces::Duration").expect("valid type name"),
        fields: vec![
            crate::dynamic::RuntimeFieldSchema::new("sec", i32::schema()),
            crate::dynamic::RuntimeFieldSchema::new("nanosec", u32::schema()),
        ],
    })
}

impl fmt::Debug for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Time")
            .field("secs", &self.since_origin.as_secs())
            .field("nanos", &self.since_origin.subsec_nanos())
            .finish()
    }
}

impl Time {
    pub fn zero() -> Self {
        Self {
            since_origin: Duration::ZERO,
        }
    }

    /// Convert a wallclock timestamp into a `Time` instant.
    pub fn from_wallclock(time: SystemTime) -> Self {
        let since_origin = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        Self { since_origin }
    }

    #[deprecated(note = "use Time::from_wallclock instead")]
    pub fn from_system_time(time: SystemTime) -> Self {
        Self::from_wallclock(time)
    }

    /// Construct a `Time` from a raw nanosecond count on the active timeline.
    pub fn from_nanos(nanos: i64) -> Self {
        let nanos = u64::try_from(nanos).unwrap_or_default();
        Self {
            since_origin: Duration::from_nanos(nanos),
        }
    }

    #[deprecated(note = "use Time::from_nanos instead")]
    pub fn from_unix_nanos(nanos: i64) -> Self {
        Self::from_nanos(nanos)
    }

    /// Interpret this instant as wallclock time.
    pub fn to_wallclock(self) -> SystemTime {
        UNIX_EPOCH + self.since_origin
    }

    #[deprecated(note = "use Time::to_wallclock instead")]
    pub fn to_system_time(self) -> SystemTime {
        self.to_wallclock()
    }

    /// Return the raw nanosecond position of this instant on its timeline.
    pub fn as_nanos(self) -> i64 {
        self.since_origin.as_nanos().min(i64::MAX as u128) as i64
    }

    #[deprecated(note = "use Time::as_nanos instead")]
    pub fn as_unix_nanos(self) -> i64 {
        self.as_nanos()
    }

    pub fn saturating_add(self, duration: Duration) -> Self {
        Self {
            since_origin: self.since_origin.saturating_add(duration),
        }
    }

    pub fn saturating_sub(self, duration: Duration) -> Self {
        Self {
            since_origin: self.since_origin.saturating_sub(duration),
        }
    }

    pub fn duration_since(self, earlier: Time) -> Duration {
        self.since_origin.saturating_sub(earlier.since_origin)
    }
}

impl From<SystemTime> for Time {
    fn from(value: SystemTime) -> Self {
        Self::from_wallclock(value)
    }
}

impl Add<Duration> for Time {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        self.saturating_add(rhs)
    }
}

impl Sub<Duration> for Time {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        self.saturating_sub(rhs)
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Debug)]
pub enum ClockError {
    NotLogical,
    TimeWentBackwards,
}

impl fmt::Display for ClockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClockError::NotLogical => write!(f, "clock is not logical"),
            ClockError::TimeWentBackwards => write!(f, "logical time cannot move backwards"),
        }
    }
}

impl std::error::Error for ClockError {}

#[derive(Clone)]
pub struct Clock {
    inner: Arc<ClockInner>,
}

enum ClockInner {
    Wallclock,
    Logical(LogicalClockState),
}

struct LogicalClockState {
    now: Mutex<Time>,
    notify: Notify,
}

impl fmt::Debug for Clock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.inner.as_ref() {
            ClockInner::Wallclock => "Wallclock",
            ClockInner::Logical(_) => "Logical",
        };

        f.debug_struct("Clock")
            .field("kind", &kind)
            .finish_non_exhaustive()
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::wallclock()
    }
}

impl Clock {
    pub fn wallclock() -> Self {
        Self {
            inner: Arc::new(ClockInner::Wallclock),
        }
    }

    #[deprecated(note = "use Clock::wallclock instead")]
    pub fn system() -> Self {
        Self::wallclock()
    }

    pub fn logical(start: Time) -> Self {
        Self {
            inner: Arc::new(ClockInner::Logical(LogicalClockState {
                now: Mutex::new(start),
                notify: Notify::new(),
            })),
        }
    }

    #[deprecated(note = "use Clock::logical instead")]
    pub fn simulated(start: Time) -> Self {
        Self::logical(start)
    }

    pub fn now(&self) -> Time {
        match self.inner.as_ref() {
            ClockInner::Wallclock => Time::from_wallclock(SystemTime::now()),
            ClockInner::Logical(state) => *state.now.lock(),
        }
    }

    pub fn set_time(&self, time: Time) -> Result<(), ClockError> {
        match self.inner.as_ref() {
            ClockInner::Wallclock => Err(ClockError::NotLogical),
            ClockInner::Logical(state) => {
                let mut current = state.now.lock();
                if time < *current {
                    return Err(ClockError::TimeWentBackwards);
                }
                *current = time;
                state.notify.notify_waiters();
                Ok(())
            }
        }
    }

    pub fn advance(&self, delta: Duration) -> Result<Time, ClockError> {
        match self.inner.as_ref() {
            ClockInner::Wallclock => Err(ClockError::NotLogical),
            ClockInner::Logical(state) => {
                let mut current = state.now.lock();
                *current = current.saturating_add(delta);
                let now = *current;
                state.notify.notify_waiters();
                Ok(now)
            }
        }
    }

    pub fn sleep_until(&self, deadline: Time) -> Sleep {
        match self.inner.as_ref() {
            ClockInner::Wallclock => {
                let now = SystemTime::now();
                let deadline = deadline.to_wallclock();
                let duration = deadline.duration_since(now).unwrap_or(Duration::ZERO);
                Sleep(Box::pin(tokio::time::sleep(duration)))
            }
            ClockInner::Logical(_) => {
                let clock = self.clone();
                Sleep(Box::pin(async move {
                    loop {
                        // Obtain and *enable* the Notified future before checking the
                        // condition.  `enable()` registers this task as a waiter
                        // immediately, so a concurrent `notify_waiters()` call that
                        // fires between the condition check and the first `.await` poll
                        // is not lost.
                        let notified = match clock.inner.as_ref() {
                            ClockInner::Wallclock => unreachable!(),
                            ClockInner::Logical(state) => state.notify.notified(),
                        };
                        tokio::pin!(notified);
                        notified.as_mut().enable();

                        if clock.now() >= deadline {
                            break;
                        }
                        notified.await;
                    }
                }))
            }
        }
    }

    pub fn sleep(&self, duration: impl Into<Duration>) -> Sleep {
        let deadline = self.now().saturating_add(duration.into());
        self.sleep_until(deadline)
    }

    pub fn interval(&self, period: impl Into<Duration>) -> Interval {
        let period = period.into();
        Interval {
            clock: self.clone(),
            period,
            next_deadline: self.now().saturating_add(period),
        }
    }

    /// Create a reusable timer tied to this clock.
    ///
    /// Unlike [`Interval`], a [`Timer`] exposes convenience methods to inspect
    /// and reset its cadence, making it a better fit for long-lived robotics
    /// tasks that need explicit periodic scheduling.
    pub fn timer(&self, period: impl Into<Duration>) -> Timer {
        Timer::new(self.clone(), period)
    }
}

pub struct Sleep(Pin<Box<dyn Future<Output = ()> + Send>>);

impl Future for Sleep {
    type Output = ();

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.0.as_mut().poll(cx)
    }
}

pub struct Interval {
    clock: Clock,
    period: Duration,
    next_deadline: Time,
}

impl Interval {
    pub async fn tick(&mut self) -> Time {
        self.clock.sleep_until(self.next_deadline).await;
        let fired_at = self.next_deadline;
        self.next_deadline = self.next_deadline.saturating_add(self.period);
        fired_at
    }
}

#[derive(Debug, Clone)]
pub struct Timer {
    clock: Clock,
    period: Duration,
    start: Time,
}

impl Timer {
    pub fn new(clock: Clock, period: impl Into<Duration>) -> Self {
        let period = period.into();
        Self {
            start: clock.now(),
            clock,
            period,
        }
    }

    pub fn period(&self) -> Duration {
        self.period
    }

    pub fn deadline(&self) -> Time {
        self.start.saturating_add(self.period)
    }

    pub fn reset(&mut self) {
        self.start = self.clock.now();
    }

    pub fn set_period(&mut self, period: impl Into<Duration>) {
        self.period = period.into();
    }

    pub async fn tick(&mut self) -> Time {
        let deadline = self.deadline();
        self.clock.sleep_until(deadline).await;
        let fired_at = deadline;
        self.start = fired_at;
        fired_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;

    #[test]
    fn wallclock_is_default() {
        let clock = Clock::default();
        assert!(matches!(
            clock.set_time(Time::zero()),
            Err(ClockError::NotLogical)
        ));
    }

    #[test]
    fn ztime_type_info_uses_trait_default_schema_hash() {
        let expected = crate::dynamic::schema_tree_hash(Time::type_name(), &Time::schema())
            .expect("Time schema should produce a hash");

        assert_eq!(Time::schema_hash(), expected);
    }

    #[tokio::test]
    async fn logical_clock_can_advance_manually() {
        let clock = Clock::logical(Time::zero());
        let mut interval = clock.interval(Duration::from_secs(1));

        let waiter = tokio::spawn(async move { interval.tick().await });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());

        clock.advance(Duration::from_secs(1)).unwrap();
        let tick = waiter.await.unwrap();
        assert_eq!(tick, Time::from_nanos(1_000_000_000));
    }

    #[tokio::test]
    async fn logical_sleep_follows_logical_time() {
        let clock = Clock::logical(Time::zero());
        let sleep = clock.sleep(Duration::from_millis(10));

        let waiter = tokio::spawn(sleep);
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());

        clock.advance(Duration::from_millis(10)).unwrap();
        waiter.await.unwrap();
    }

    #[tokio::test]
    async fn logical_ztimer_follows_logical_time() {
        let clock = Clock::logical(Time::zero());
        let mut timer = clock.timer(Duration::from_millis(10));

        let waiter = tokio::spawn(async move { timer.tick().await });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());

        clock.advance(Duration::from_millis(10)).unwrap();
        let tick = waiter.await.unwrap();
        assert_eq!(tick, Time::from_nanos(10_000_000));
    }

    #[test]
    fn ztimer_reset_uses_current_clock_time() {
        let clock = Clock::logical(Time::zero());
        let mut timer = clock.timer(Duration::from_secs(2));
        assert_eq!(timer.deadline(), Time::from_nanos(2_000_000_000));

        clock.advance(Duration::from_secs(5)).unwrap();
        timer.reset();

        assert_eq!(timer.deadline(), Time::from_nanos(7_000_000_000));
    }

    #[test]
    fn ztimer_set_period_before_first_tick_preserves_creation_anchor() {
        let clock = Clock::logical(Time::zero());
        let mut timer = clock.timer(Duration::from_secs(2));

        timer.set_period(Duration::from_secs(5));

        assert_eq!(timer.deadline(), Time::from_nanos(5_000_000_000));
    }

    #[tokio::test]
    async fn ztimer_set_period_after_tick_preserves_last_fire_phase() {
        let clock = Clock::logical(Time::zero());
        let mut timer = clock.timer(Duration::from_secs(2));

        let waiter = tokio::spawn(async move {
            let first_tick = timer.tick().await;
            timer.set_period(Duration::from_secs(5));
            (first_tick, timer.deadline())
        });

        tokio::task::yield_now().await;
        clock.advance(Duration::from_secs(2)).unwrap();

        let (first_tick, next_deadline) = waiter.await.unwrap();
        assert_eq!(first_tick, Time::from_nanos(2_000_000_000));
        assert_eq!(next_deadline, Time::from_nanos(7_000_000_000));
    }

    #[tokio::test]
    async fn logical_sleep_no_lost_wakeup_when_advance_before_poll() {
        // Regression test: advance the clock past the deadline *before* the sleep
        // future is ever polled.  Without the enable() fix this would hang forever
        // because notify_waiters() fires before the future registers as a waiter.
        let clock = Clock::logical(Time::zero());
        let sleep = clock.sleep(Duration::from_millis(10));
        // Advance BEFORE yielding — the future has not been polled yet.
        clock.advance(Duration::from_millis(10)).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), sleep)
            .await
            .expect("sleep should resolve without hanging");
    }

    // --- Time ---

    #[test]
    fn ztime_zero_and_default() {
        assert_eq!(Time::zero(), Time::default());
        assert_eq!(Time::zero().as_nanos(), 0);
    }

    #[test]
    fn ztime_from_nanos_negative_clamps_to_zero() {
        assert_eq!(Time::from_nanos(-1).as_nanos(), 0);
    }

    #[test]
    fn ztime_from_wallclock_roundtrip() {
        let t = Time::from_nanos(1_000_000_000);
        let sys = t.to_wallclock();
        let back = Time::from_wallclock(sys);
        assert_eq!(back, t);
    }

    #[test]
    fn ztime_saturating_add_sub() {
        let t = Time::from_nanos(5_000_000_000);
        let d = Duration::from_secs(2);
        assert_eq!(t.saturating_add(d).as_nanos(), 7_000_000_000);
        assert_eq!(t.saturating_sub(d).as_nanos(), 3_000_000_000);
        // sub below zero saturates
        assert_eq!(Time::zero().saturating_sub(d).as_nanos(), 0);
    }

    #[test]
    fn ztime_duration_since() {
        let a = Time::from_nanos(5_000_000_000);
        let b = Time::from_nanos(3_000_000_000);
        assert_eq!(a.duration_since(b), Duration::from_secs(2));
        // saturates to zero when earlier > self
        assert_eq!(b.duration_since(a), Duration::ZERO);
    }

    // --- Clock constructors ---

    #[test]
    fn zclock_wallclock_constructor() {
        let c = Clock::wallclock();
        assert!(matches!(
            c.set_time(Time::zero()),
            Err(ClockError::NotLogical)
        ));
    }

    #[test]
    #[allow(deprecated)]
    fn legacy_time_aliases_still_work() {
        let t = Time::from_system_time(SystemTime::UNIX_EPOCH + Duration::from_secs(1));
        assert_eq!(t.as_unix_nanos(), 1_000_000_000);
        assert_eq!(
            t.to_system_time(),
            SystemTime::UNIX_EPOCH + Duration::from_secs(1)
        );
        assert_eq!(Time::from_unix_nanos(5).as_nanos(), 5);

        assert!(matches!(
            Clock::system().set_time(Time::zero()),
            Err(ClockError::NotLogical)
        ));
        assert_eq!(Clock::simulated(Time::zero()).now(), Time::zero());
    }

    #[test]
    fn wallclock_now_is_nonzero() {
        let t = Clock::wallclock().now();
        assert!(t.as_nanos() > 0);
    }

    #[test]
    fn ztime_type_info_uses_native_schema_type_name() {
        assert_eq!(Time::type_name(), "ros_z::Time");
        let schema = Time::schema();
        let crate::dynamic::TypeShape::Struct { name, .. } = schema.as_ref() else {
            panic!("expected time struct schema");
        };
        assert_eq!(name.as_str(), "ros_z::Time");
        assert_eq!(Time::type_info().name, "ros_z::Time");
    }

    #[test]
    fn ztime_duration_field_uses_native_nested_type_name() {
        let schema = Time::schema();
        let crate::dynamic::TypeShape::Struct { fields, .. } = schema.as_ref() else {
            panic!("expected time struct schema");
        };
        let crate::dynamic::TypeShape::Struct { name, .. } = fields[0].schema.as_ref() else {
            panic!("expected nested duration schema");
        };

        assert_eq!(name.as_str(), "builtin_interfaces::Duration");
    }

    // --- set_time ---

    #[test]
    fn set_time_advances_logical_clock() {
        let clock = Clock::logical(Time::zero());
        let t = Time::from_nanos(1_000_000_000);
        clock.set_time(t).unwrap();
        assert_eq!(clock.now(), t);
    }

    #[test]
    fn set_time_rejects_backwards() {
        let clock = Clock::logical(Time::from_nanos(1_000_000_000));
        let err = clock.set_time(Time::zero()).unwrap_err();
        assert!(matches!(err, ClockError::TimeWentBackwards));
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn set_time_on_wallclock_errors() {
        let err = Clock::wallclock().set_time(Time::zero()).unwrap_err();
        assert!(matches!(err, ClockError::NotLogical));
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn advance_on_wallclock_errors() {
        let err = Clock::wallclock()
            .advance(Duration::from_secs(1))
            .unwrap_err();
        assert!(matches!(err, ClockError::NotLogical));
    }

    // --- wallclock sleep (just verify it doesn't block) ---

    #[tokio::test]
    async fn wallclock_sleep_zero_completes() {
        Clock::wallclock().sleep(Duration::default()).await;
    }
}
