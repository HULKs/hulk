use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use zenoh::Result;

use crate::EndpointGlobalId;

// Event kinds reported for ros-z endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndpointEventKind {
    RequestedQosIncompatible = 0,
    OfferedQosIncompatible = 1,
    MessageLost = 2,
    SubscriptionMatched = 3,
    PublicationMatched = 4,
    SubscriptionIncompatibleType = 5,
    PublisherIncompatibleType = 6,
    OfferedDeadlineMissed = 7,
    RequestedDeadlineMissed = 8,
    LivelinessLost = 9,
    LivelinessChanged = 10,
}

pub const ENDPOINT_EVENT_KIND_COUNT: usize = 11;

// Event status for a ros-z endpoint.
#[derive(Debug, Clone, Default)]
pub struct EndpointEventStatus {
    pub total_count: i32,
    pub total_count_change: i32,
    pub current_count: i32,
    pub current_count_change: i32,
    pub data: String,
    pub changed: bool,
    pub last_policy_kind: u32, // RMW QoS policy kind that caused incompatibility
}

// Event callback type
pub type EventCallback = Arc<dyn Fn(i32) + Send + Sync>;
pub type EventNotification = (EventCallback, i32);

// EventsManager - manages event state for a single publisher/subscription.
// It performs no internal locking; owners that wrap it in a Mutex must clone
// callbacks out and invoke them after releasing that outer lock.
pub struct EventsManager {
    event_statuses: Vec<EndpointEventStatus>,
    event_callbacks: Vec<Option<EventCallback>>,
    entity_global_id: EndpointGlobalId,
}

impl EventsManager {
    pub fn new(entity_global_id: EndpointGlobalId) -> Self {
        let mut event_callbacks = Vec::with_capacity(ENDPOINT_EVENT_KIND_COUNT);
        for _ in 0..ENDPOINT_EVENT_KIND_COUNT {
            event_callbacks.push(None);
        }
        Self {
            event_statuses: vec![EndpointEventStatus::default(); ENDPOINT_EVENT_KIND_COUNT],
            event_callbacks,
            entity_global_id,
        }
    }

    pub fn set_callback<F>(
        &mut self,
        event_type: EndpointEventKind,
        callback: F,
    ) -> Option<EventNotification>
    where
        F: Fn(i32) + Send + Sync + 'static,
    {
        let callback = Arc::new(callback);
        self.set_callback_arc(event_type, callback)
    }

    pub fn set_callback_arc(
        &mut self,
        event_type: EndpointEventKind,
        callback: EventCallback,
    ) -> Option<EventNotification> {
        let event_id = event_type as usize;

        // If there are unread events, trigger the callback immediately
        let unread_count = self.event_statuses[event_id].total_count_change;
        let unread_callback = (unread_count != 0).then(|| (callback.clone(), unread_count));
        if unread_count != 0 {
            self.event_statuses[event_id].total_count_change = 0;
        }

        self.event_callbacks[event_id] = Some(callback);
        unread_callback
    }

    pub fn update_event_status(
        &mut self,
        event_type: EndpointEventKind,
        change: i32,
    ) -> Option<EventNotification> {
        self.update_event_status_with_policy(event_type, change, 0)
    }

    pub fn update_event_status_with_policy(
        &mut self,
        event_type: EndpointEventKind,
        change: i32,
        policy_kind: u32,
    ) -> Option<EventNotification> {
        let event_id = event_type as usize;

        let status = &mut self.event_statuses[event_id];

        status.total_count += change.max(0);
        status.total_count_change += change.max(0);
        status.current_count = (status.current_count + change).max(0);
        status.current_count_change += change;
        status.changed = true;
        // Update policy kind if provided (non-zero for QoS incompatibility events)
        if policy_kind != 0 {
            status.last_policy_kind = policy_kind;
        }

        // Trigger callback if registered
        self.event_callbacks[event_id]
            .clone()
            .map(|callback| (callback, change))
    }

    pub fn take_event_status(&mut self, event_type: EndpointEventKind) -> EndpointEventStatus {
        let event_id = event_type as usize;

        let status = self.event_statuses[event_id].clone();
        // Reset change counters
        self.event_statuses[event_id].current_count_change = 0;
        self.event_statuses[event_id].total_count_change = 0;
        self.event_statuses[event_id].changed = false;

        status
    }

    pub fn entity_global_id(&self) -> &EndpointGlobalId {
        &self.entity_global_id
    }
}

// Callback type for triggering graph guard conditions
pub type GraphGuardConditionTrigger = Arc<dyn Fn(*mut std::ffi::c_void) + Send + Sync>;

// GraphCache event integration
pub struct GraphEventManager {
    event_callbacks: Mutex<HashMap<EndpointGlobalId, HashMap<EndpointEventKind, EventCallback>>>,
    entity_topics: Mutex<HashMap<EndpointGlobalId, String>>, // Topic name per registered entity
    // Raw guard-condition pointers are snapshotted before trigger callbacks run.
    // Unregister prevents future snapshots but cannot cancel a trigger already in flight;
    // callers must keep guard conditions alive while graph changes may be dispatched.
    graph_guard_conditions: Mutex<Vec<usize>>, // Pointers as usize for Send
    trigger_guard_condition: Mutex<Option<GraphGuardConditionTrigger>>,
}

impl Default for GraphEventManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphEventManager {
    pub fn new() -> Self {
        Self {
            event_callbacks: Mutex::new(HashMap::new()),
            entity_topics: Mutex::new(HashMap::new()),
            graph_guard_conditions: Mutex::new(Vec::new()),
            trigger_guard_condition: Mutex::new(None),
        }
    }

    pub fn set_guard_condition_trigger(&self, trigger: GraphGuardConditionTrigger) {
        *self.trigger_guard_condition.lock().unwrap() = Some(trigger);
    }

    pub fn register_event_callback<F>(
        &self,
        entity_global_id: EndpointGlobalId,
        topic: String,
        event_type: EndpointEventKind,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(i32) + Send + Sync + 'static,
    {
        let mut topics = self.entity_topics.lock().unwrap();
        topics.insert(entity_global_id, topic);
        let mut callbacks = self.event_callbacks.lock().unwrap();
        let entity_callbacks = callbacks.entry(entity_global_id).or_default();
        entity_callbacks.insert(event_type, Arc::new(callback));

        Ok(())
    }

    pub fn unregister_entity(&self, entity_global_id: &EndpointGlobalId) {
        let mut topics = self.entity_topics.lock().unwrap();
        topics.remove(entity_global_id);
        let mut callbacks = self.event_callbacks.lock().unwrap();
        callbacks.remove(entity_global_id);
    }

    pub fn register_graph_guard_condition(&self, guard_condition: *mut std::ffi::c_void) {
        let mut conditions = self.graph_guard_conditions.lock().unwrap();
        conditions.push(guard_condition as usize);
    }

    pub fn unregister_graph_guard_condition(&self, guard_condition: *mut std::ffi::c_void) {
        let mut conditions = self.graph_guard_conditions.lock().unwrap();
        let gc_usize = guard_condition as usize;
        conditions.retain(|&gc| gc != gc_usize);
    }

    pub fn trigger_event(
        &self,
        entity_global_id: &EndpointGlobalId,
        event_type: EndpointEventKind,
        change: i32,
    ) {
        self.trigger_event_with_policy(entity_global_id, event_type, change, 0);
    }

    pub fn trigger_event_with_policy(
        &self,
        entity_global_id: &EndpointGlobalId,
        event_type: EndpointEventKind,
        change: i32,
        policy_kind: u32,
    ) {
        // For QoS incompatibility events, we need to pass policy_kind through a different mechanism
        // since callbacks only take i32. We'll encode it in the change parameter's upper bits for now.
        // This is a workaround - ideally we'd change the callback signature.
        let encoded_change = if policy_kind != 0
            && (matches!(
                event_type,
                EndpointEventKind::RequestedQosIncompatible
                    | EndpointEventKind::OfferedQosIncompatible
            )) {
            // Encode policy_kind in upper 16 bits, change in lower 16 bits
            // This works because change is always small (number of incompatible entities)
            ((policy_kind as i32) << 16) | (change & 0xFFFF)
        } else {
            change
        };

        let callback = self
            .event_callbacks
            .lock()
            .unwrap()
            .get(entity_global_id)
            .and_then(|entity_callbacks| entity_callbacks.get(&event_type).cloned());

        if let Some(callback) = callback {
            callback(encoded_change);
        }
    }

    pub fn trigger_graph_change(
        &self,
        entity: &crate::entity::Entity,
        appeared: bool,
        _local_zid: zenoh::session::ZenohId,
    ) {
        use crate::entity::EndpointKind;

        let change = if appeared { 1 } else { -1 };

        // Trigger graph guard conditions for ALL graph changes (local and remote)
        let trigger = self.trigger_guard_condition.lock().unwrap().clone();
        if let Some(trigger) = trigger {
            let guard_conditions = self.graph_guard_conditions.lock().unwrap().clone();
            for gc_usize in guard_conditions {
                trigger(gc_usize as *mut std::ffi::c_void);
            }
        }

        // Determine which event kind based on entity kind
        // When a publisher appears/disappears, subscriptions get SubscriptionMatched events
        // When a subscription appears/disappears, publishers get PublicationMatched events
        let event_type = match entity {
            crate::entity::Entity::Endpoint(endpoint) => match endpoint.kind {
                EndpointKind::Publisher => EndpointEventKind::SubscriptionMatched,
                EndpointKind::Subscription => EndpointEventKind::PublicationMatched,
                EndpointKind::Service => return, // TODO: Add service matched events
                EndpointKind::Client => return,  // TODO: Add service matched events
            },
            crate::entity::Entity::Node(_) => return, // Node changes don't trigger matched events
        };

        // Find all entities on the same topic that should be notified
        let changed_topic = match entity {
            crate::entity::Entity::Endpoint(endpoint) => &endpoint.topic,
            _ => return,
        };

        let callbacks = {
            let entity_topics = self.entity_topics.lock().unwrap();
            let callbacks = self.event_callbacks.lock().unwrap();
            callbacks
                .iter()
                .filter_map(|(entity_global_id, entity_callbacks)| {
                    // Only notify entities on the same topic
                    if let Some(registered_topic) = entity_topics.get(entity_global_id)
                        && registered_topic == changed_topic
                    {
                        entity_callbacks.get(&event_type).cloned()
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };

        for callback in callbacks {
            callback(change);
        }
    }
}

// Wait set integration (simplified)
pub struct EventWaitData {
    pub triggered: AtomicBool,
    // TODO: Add condition variable for proper waiting
}

impl Default for EventWaitData {
    fn default() -> Self {
        Self::new()
    }
}

impl EventWaitData {
    pub fn new() -> Self {
        Self {
            triggered: AtomicBool::new(false),
        }
    }

    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::Acquire)
    }

    pub fn set_triggered(&self, triggered: bool) {
        self.triggered.store(triggered, Ordering::Release);
    }
}

// Endpoint event handle.
pub struct EventHandle {
    pub events_mgr: Arc<Mutex<EventsManager>>,
    pub event_type: EndpointEventKind,
}

impl std::fmt::Debug for EventHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventHandle")
            .field("event_type", &self.event_type)
            .finish()
    }
}

// EventHandle is Send because the Arc<Mutex<>> provides thread safety.
unsafe impl Send for EventHandle {}

impl EventHandle {
    pub fn new(events_mgr: Arc<Mutex<EventsManager>>, event_type: EndpointEventKind) -> Self {
        Self {
            events_mgr,
            event_type,
        }
    }

    pub fn take_event(&self) -> EndpointEventStatus {
        let mut mgr = self.events_mgr.lock().unwrap();
        mgr.take_event_status(self.event_type)
    }

    pub fn is_ready(&self) -> bool {
        let mgr = self.events_mgr.lock().unwrap();
        mgr.event_statuses[self.event_type as usize].changed
    }

    pub fn set_callback<F>(&self, callback: F)
    where
        F: Fn(i32) + Send + Sync + 'static,
    {
        let callback = Arc::new(callback);
        let notification = {
            let mut mgr = self.events_mgr.lock().unwrap();
            mgr.set_callback_arc(self.event_type, callback.clone())
        };
        if let Some((callback, unread_count)) = notification {
            callback(unread_count);
        }
    }

    pub fn update_event_status(&self, change: i32) {
        let notification = {
            let mut mgr = self.events_mgr.lock().unwrap();
            mgr.update_event_status(self.event_type, change)
        };
        if let Some((callback, change)) = notification {
            callback(change);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint_global_id(n: u8) -> EndpointGlobalId {
        let mut endpoint_global_id = [0u8; 16];
        endpoint_global_id[0] = n;
        endpoint_global_id
    }

    // ── EventsManager ────────────────────────────────────────────────────────

    #[test]
    fn test_events_manager_initial_state() {
        let mgr = EventsManager::new(endpoint_global_id(1));
        // All callbacks are None; take_event_status returns zeroed status
        let status = {
            let mut m = mgr;
            m.take_event_status(EndpointEventKind::PublicationMatched)
        };
        assert!(!status.changed);
        assert_eq!(status.total_count, 0);
        assert_eq!(status.current_count, 0);
    }

    #[test]
    fn test_update_event_status_fires_callback() {
        let called = Arc::new(Mutex::new(0i32));
        let called_clone = called.clone();

        let mut mgr = EventsManager::new(endpoint_global_id(2));
        let _ = mgr.set_callback(EndpointEventKind::SubscriptionMatched, move |change| {
            *called_clone.lock().unwrap() += change;
        });

        if let Some((callback, change)) =
            mgr.update_event_status(EndpointEventKind::SubscriptionMatched, 1)
        {
            callback(change);
        }
        assert_eq!(*called.lock().unwrap(), 1);

        if let Some((callback, change)) =
            mgr.update_event_status(EndpointEventKind::SubscriptionMatched, 1)
        {
            callback(change);
        }
        assert_eq!(*called.lock().unwrap(), 2);
    }

    #[test]
    fn test_update_without_callback_no_panic() {
        let mut mgr = EventsManager::new(endpoint_global_id(3));
        // No callback registered — must not panic
        let _ = mgr.update_event_status(EndpointEventKind::MessageLost, 1);
        let status = mgr.take_event_status(EndpointEventKind::MessageLost);
        assert!(status.changed);
        assert_eq!(status.total_count, 1);
    }

    #[test]
    fn test_set_callback_fires_immediately_for_unread_events() {
        let mut mgr = EventsManager::new(endpoint_global_id(4));
        // Accumulate events before any callback is registered
        let _ = mgr.update_event_status(EndpointEventKind::PublicationMatched, 3);

        let fired = Arc::new(Mutex::new(0i32));
        let fired_clone = fired.clone();
        // Registering the callback now should fire immediately with the backlog
        if let Some((callback, change)) =
            mgr.set_callback(EndpointEventKind::PublicationMatched, move |change| {
                *fired_clone.lock().unwrap() += change;
            })
        {
            callback(change);
        }

        assert_eq!(*fired.lock().unwrap(), 3);
    }

    #[test]
    fn test_set_callback_replaces_existing() {
        let old_fired = Arc::new(Mutex::new(false));
        let new_fired = Arc::new(Mutex::new(false));

        let old_clone = old_fired.clone();
        let new_clone = new_fired.clone();

        let mut mgr = EventsManager::new(endpoint_global_id(5));
        let _ = mgr.set_callback(EndpointEventKind::LivelinessLost, move |_| {
            *old_clone.lock().unwrap() = true;
        });
        let _ = mgr.set_callback(EndpointEventKind::LivelinessLost, move |_| {
            *new_clone.lock().unwrap() = true;
        });

        if let Some((callback, change)) =
            mgr.update_event_status(EndpointEventKind::LivelinessLost, 1)
        {
            callback(change);
        }
        assert!(!*old_fired.lock().unwrap(), "old callback must not fire");
        assert!(*new_fired.lock().unwrap(), "new callback must fire");
    }

    #[test]
    fn test_take_event_status_resets_change_counters() {
        let mut mgr = EventsManager::new(endpoint_global_id(6));
        let _ = mgr.update_event_status(EndpointEventKind::RequestedQosIncompatible, 2);

        let first = mgr.take_event_status(EndpointEventKind::RequestedQosIncompatible);
        assert!(first.changed);
        assert_eq!(first.total_count_change, 2);

        // Second take: change counters must be reset, total count persists
        let second = mgr.take_event_status(EndpointEventKind::RequestedQosIncompatible);
        assert!(!second.changed);
        assert_eq!(second.total_count_change, 0);
        assert_eq!(second.total_count, 2); // cumulative count unchanged
    }

    #[test]
    fn endpoint_event_api_uses_endpoint_names() {
        let mut manager = EventsManager::new(endpoint_global_id(7));

        let _ = manager.update_event_status(EndpointEventKind::MessageLost, 1);
        let status: EndpointEventStatus = manager.take_event_status(EndpointEventKind::MessageLost);

        assert_eq!(status.total_count, 1);
    }

    #[test]
    fn test_update_with_policy_sets_last_policy_kind() {
        let mut mgr = EventsManager::new(endpoint_global_id(7));
        let _ =
            mgr.update_event_status_with_policy(EndpointEventKind::OfferedQosIncompatible, 1, 42);
        let status = mgr.take_event_status(EndpointEventKind::OfferedQosIncompatible);
        assert_eq!(status.last_policy_kind, 42);
    }

    // ── GraphEventManager ────────────────────────────────────────────────────

    #[test]
    fn graph_event_callback_can_unregister_entity_without_deadlock() {
        use std::sync::mpsc;
        use std::time::Duration;

        let manager = Arc::new(GraphEventManager::new());
        let endpoint_global_id = endpoint_global_id(42);
        let callback_manager = manager.clone();
        let (done_tx, done_rx) = mpsc::channel();

        manager
            .register_event_callback(
                endpoint_global_id,
                "/topic".to_string(),
                EndpointEventKind::SubscriptionMatched,
                move |_| {
                    callback_manager.unregister_entity(&endpoint_global_id);
                    done_tx.send(()).unwrap();
                },
            )
            .unwrap();

        let trigger_manager = manager.clone();
        std::thread::spawn(move || {
            trigger_manager.trigger_event(
                &endpoint_global_id,
                EndpointEventKind::SubscriptionMatched,
                1,
            );
        });

        done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("callback did not complete");
    }

    #[test]
    fn event_handle_immediate_callback_can_take_event_without_deadlock() {
        use std::sync::mpsc;
        use std::time::Duration;

        let events_mgr = Arc::new(Mutex::new(EventsManager::new(endpoint_global_id(43))));
        let _ = events_mgr
            .lock()
            .unwrap()
            .update_event_status(EndpointEventKind::LivelinessChanged, 3);
        let handle = Arc::new(EventHandle::new(
            events_mgr,
            EndpointEventKind::LivelinessChanged,
        ));
        let callback_handle = handle.clone();
        let (done_tx, done_rx) = mpsc::channel();

        std::thread::spawn(move || {
            handle.set_callback(move |_| {
                let _ = callback_handle.take_event();
                done_tx.send(()).unwrap();
            });
        });

        done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("callback did not complete");
    }

    #[test]
    fn test_graph_event_manager_register_and_trigger() {
        let mgr = GraphEventManager::new();
        let fired = Arc::new(Mutex::new(0i32));
        let fired_clone = fired.clone();

        mgr.register_event_callback(
            endpoint_global_id(1),
            "/test".to_string(),
            EndpointEventKind::SubscriptionMatched,
            move |v| {
                *fired_clone.lock().unwrap() += v;
            },
        )
        .unwrap();

        mgr.trigger_event(
            &endpoint_global_id(1),
            EndpointEventKind::SubscriptionMatched,
            5,
        );
        assert_eq!(*fired.lock().unwrap(), 5);
    }

    #[test]
    fn test_graph_event_manager_unregister_stops_firing() {
        let mgr = GraphEventManager::new();
        let fired = Arc::new(Mutex::new(0i32));
        let fired_clone = fired.clone();

        mgr.register_event_callback(
            endpoint_global_id(2),
            "/test".to_string(),
            EndpointEventKind::PublicationMatched,
            move |v| {
                *fired_clone.lock().unwrap() += v;
            },
        )
        .unwrap();

        mgr.trigger_event(
            &endpoint_global_id(2),
            EndpointEventKind::PublicationMatched,
            1,
        );
        assert_eq!(*fired.lock().unwrap(), 1);

        mgr.unregister_entity(&endpoint_global_id(2));
        mgr.trigger_event(
            &endpoint_global_id(2),
            EndpointEventKind::PublicationMatched,
            1,
        );
        assert_eq!(*fired.lock().unwrap(), 1); // unchanged
    }

    #[test]
    fn test_graph_event_manager_no_callback_no_panic() {
        let mgr = GraphEventManager::new();
        // Trigger on an unregistered GID — must not panic
        mgr.trigger_event(
            &endpoint_global_id(99),
            EndpointEventKind::LivelinessChanged,
            1,
        );
    }

    // ── EventWaitData ────────────────────────────────────────────────────────

    #[test]
    fn test_event_wait_data_set_and_check() {
        let w = EventWaitData::new();
        assert!(!w.is_triggered());
        w.set_triggered(true);
        assert!(w.is_triggered());
        w.set_triggered(false);
        assert!(!w.is_triggered());
    }

    // ── EventHandle ──────────────────────────────────────────────────────────

    #[test]
    fn test_event_handle_is_ready_and_take() {
        let mgr = Arc::new(Mutex::new(EventsManager::new(endpoint_global_id(8))));
        let handle = EventHandle::new(mgr.clone(), EndpointEventKind::MessageLost);

        assert!(!handle.is_ready());

        let _ = mgr
            .lock()
            .unwrap()
            .update_event_status(EndpointEventKind::MessageLost, 2);

        assert!(handle.is_ready());
        let status = handle.take_event();
        assert_eq!(status.total_count, 2);
        assert!(!handle.is_ready()); // reset after take
    }

    #[test]
    fn test_event_handle_set_callback() {
        let mgr = Arc::new(Mutex::new(EventsManager::new(endpoint_global_id(9))));
        let handle = EventHandle::new(mgr.clone(), EndpointEventKind::LivelinessChanged);
        let fired = Arc::new(Mutex::new(0i32));
        let fired_clone = fired.clone();

        handle.set_callback(move |v| {
            *fired_clone.lock().unwrap() += v;
        });

        handle.update_event_status(3);
        assert_eq!(*fired.lock().unwrap(), 3);
    }
}
