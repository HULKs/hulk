use super::*;
use plan::{Evaluation, Expr, Frame, Sample, State, Value};

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

type LocalSnapshot = dyn Fn() -> Arc<PlanSnapshot> + Send + Sync;

#[derive(Clone)]
pub struct PlanObservation {
    cursor: CursorHandle,
    receiver: watch::Receiver<Arc<PlanSnapshot>>,
    updates: watch::Receiver<u64>,
    pub(super) local: Option<Arc<LocalSnapshot>>,
}

impl PlanObservation {
    pub fn snapshot(&self) -> Arc<PlanSnapshot> {
        self.local
            .as_ref()
            .map_or_else(|| self.receiver.borrow().clone(), |snapshot| snapshot())
    }
    pub fn status(&self) -> PlanStatus {
        let snapshot = self.snapshot();
        PlanStatus {
            current: self.is_current(&snapshot),
            snapshot,
        }
    }
    pub fn is_current(&self, snapshot: &PlanSnapshot) -> bool {
        self.is_current_revision(snapshot.revision)
    }
    pub(crate) fn is_current_revision(&self, revision: u64) -> bool {
        revision == self.cursor.position().revision
    }
    /// Notifications invalidate the last read. Live evaluates on demand while
    /// ingestion and index maintenance continue independently in the background.
    pub fn subscribe_updates(&self) -> watch::Receiver<u64> {
        self.updates.clone()
    }
    pub(crate) fn channel(cursor: CursorHandle) -> (Self, PlanSender) {
        let (sender, receiver) = watch::channel(Arc::new(PlanSnapshot {
            revision: cursor.position().revision,
            evaluation: Arc::new(Evaluation::unresolved(Time::zero(), State::Pending)),
        }));
        let (updates_sender, updates) = watch::channel(0);
        (
            Self {
                cursor,
                receiver,
                updates,
                local: None,
            },
            PlanSender {
                sender,
                updates: updates_sender,
            },
        )
    }
}

pub(crate) struct PlanSender {
    sender: watch::Sender<Arc<PlanSnapshot>>,
    updates: watch::Sender<u64>,
}
impl PlanSender {
    pub fn notify(&self) {
        self.updates
            .send_modify(|revision| *revision = revision.wrapping_add(1));
    }
    pub fn send_replace(&self, value: Arc<PlanSnapshot>) {
        self.sender.send_replace(value);
        self.notify();
    }
    pub fn borrow(&self) -> watch::Ref<'_, Arc<PlanSnapshot>> {
        self.sender.borrow()
    }
    pub async fn closed(&self) {
        self.sender.closed().await;
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
