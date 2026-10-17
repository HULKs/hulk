use super::*;
use plan::{Evaluation, Expr, Frame, Sample, SelectionExpr, State, Value};

#[derive(Clone, Debug)]
pub struct PlanSnapshot {
    pub revision: u64,
    pub evaluation: Arc<Evaluation>,
}

/// Status is derived on demand, without retaining a second result representation.
#[derive(Clone, Debug)]
pub struct PlanStatus {
    pub current: bool,
    pub snapshot: Arc<PlanSnapshot>,
}

struct ReadSnapshot {
    read: Box<dyn Fn() -> Arc<PlanSnapshot> + Send + Sync>,
    cancel: tokio_util::sync::CancellationToken,
}

impl Drop for ReadSnapshot {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

#[derive(Clone)]
pub struct PlanObservation {
    pub(crate) cursor: CursorHandle,
    updates: watch::Receiver<u64>,
    read: Arc<ReadSnapshot>,
}

impl PlanObservation {
    /// Construct an observation backed by an immutable-snapshot reader.
    /// Notify `updates` after changing results; stamp snapshots with the cursor
    /// revision captured before evaluation, so seeks invalidate obsolete work.
    /// Producers should stop when [`Self::cancellation_token`] is cancelled.
    pub fn new(
        cursor: CursorHandle,
        read: impl Fn() -> Arc<PlanSnapshot> + Send + Sync + 'static,
        updates: watch::Receiver<u64>,
    ) -> Self {
        Self {
            cursor,
            updates,
            read: Arc::new(ReadSnapshot {
                read: Box::new(read),
                cancel: tokio_util::sync::CancellationToken::new(),
            }),
        }
    }

    /// Cancelled when the last observation is dropped, even if update receivers remain.
    pub fn cancellation_token(&self) -> tokio_util::sync::CancellationToken {
        self.read.cancel.clone()
    }

    pub fn snapshot(&self) -> Arc<PlanSnapshot> {
        (self.read.read)()
    }
    pub fn status(&self) -> PlanStatus {
        let snapshot = self.snapshot();
        PlanStatus {
            current: self.is_current(&snapshot),
            snapshot,
        }
    }
    pub fn is_current(&self, snapshot: &PlanSnapshot) -> bool {
        snapshot.revision == self.cursor.position().revision
    }
    /// Notifications invalidate the last read. Live evaluates on demand while
    /// ingestion and index maintenance continue independently in the background.
    pub fn subscribe_updates(&self) -> watch::Receiver<u64> {
        self.updates.clone()
    }
}

mod sealed {
    pub trait Sealed {}
}
pub trait PlanValue: sealed::Sealed + Clone {
    fn from_value(value: &Value) -> Result<Self>;
}
macro_rules! value {
    ($ty:ty,$pat:pat => $body:expr) => {
        impl sealed::Sealed for $ty {}
        impl PlanValue for $ty {
            fn from_value(value: &Value) -> Result<Self> {
                match value {
                    $pat => $body,
                    _ => Err(query_error("plan result type mismatch")),
                }
            }
        }
    };
}
value!(Option<Time>,Value::Time(time)=>Ok(*time));
value!(Vec<Time>,Value::Times(times)=>Ok(times.clone()));
value!(Vec<Sample>,Value::Records(records)=>Ok(records.clone()));
value!(Option<Sample>,Value::Records(records)=>if records.len()<=1 {Ok(records.first().cloned())} else {Err(query_error("expected one sample"))});
value!(Vec<Frame>,Value::Frames(frames)=>Ok(frames.clone()));
value!(Option<Frame>,Value::Frames(frames)=>if frames.len()<=1 {Ok(frames.first().cloned())} else {Err(query_error("expected one frame"))});

pub struct TypedPlanObservation<T> {
    raw: PlanObservation,
    marker: std::marker::PhantomData<fn() -> T>,
}
pub struct TypedPlanSnapshot<T> {
    pub state: State<T>,
    pub raw: Arc<PlanSnapshot>,
}
impl<T: PlanValue> TypedPlanObservation<T> {
    pub fn snapshot(&self) -> TypedPlanSnapshot<T> {
        let raw = self.raw.snapshot();
        TypedPlanSnapshot {
            state: self.value(&raw),
            raw,
        }
    }
    pub fn raw(&self) -> &PlanObservation {
        &self.raw
    }
    fn value(&self, snapshot: &PlanSnapshot) -> State<T> {
        if !self.raw.is_current(snapshot) {
            return State::Pending;
        }
        match &snapshot.evaluation.state {
            State::Pending => State::Pending,
            State::Failed(error) => State::Failed(error.clone()),
            State::Ready(value) => match T::from_value(value) {
                Ok(value) => State::Ready(value),
                Err(error) => State::Failed(error.to_string()),
            },
        }
    }
}
impl<T: PlanValue> Expr<T> {
    pub fn observe(
        self,
        backend: Arc<dyn Backend>,
        cursor: CursorHandle,
    ) -> Result<TypedPlanObservation<T>> {
        Ok(TypedPlanObservation {
            raw: backend.observe(cursor, self.plan())?,
            marker: std::marker::PhantomData,
        })
    }
}

impl SelectionExpr {
    pub fn observe(
        self,
        backend: Arc<dyn Backend>,
        cursor: CursorHandle,
    ) -> Result<TypedPlanObservation<Vec<Sample>>> {
        self.into_expr().observe(backend, cursor)
    }
}
