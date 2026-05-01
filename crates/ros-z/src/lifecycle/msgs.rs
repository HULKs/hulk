//! Lifecycle wire types defined inline to avoid the ros-z → ros-z-msgs circular dependency.
//!
//! `ros-z-msgs` depends on `ros-z` (it uses `WireMessage`, `Service`, etc.), so `ros-z` cannot
//! depend on `ros-z-msgs` at library level — only as a dev-dependency for tests and examples.
//! Lifecycle nodes need lifecycle types both at library level (the service servers and
//! the transition-event publisher live inside `ros-z`) and in user code, so we define the wire
//! types here instead of re-exporting them from `ros-z-msgs`.
//!
//! # Schema hashes
//!
//! These inline lifecycle types cannot flow through the normal generated-code pipeline because
//! `ros-z` cannot depend on `ros-z-msgs` at library level. We compute their hashes
//! directly from the runtime message schemas and service descriptors instead.

use ros_z_cdr::{CdrBuffer, CdrDecode, CdrEncode, CdrEncodedSize, CdrReader, CdrWriter};
use ros_z_schema::{ServiceDef, compute_hash};

use crate::{
    Message, ServiceTypeInfo,
    dynamic::{FieldType, MessageSchema},
    entity::{SchemaHash, TypeInfo},
    msg::{GeneratedCdrCodec, Service},
};

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct LcState {
    pub id: u8,
    pub label: String,
}

impl Message for LcState {
    type Codec = GeneratedCdrCodec<Self>;

    fn type_name() -> &'static str {
        "ros_z::lifecycle::State"
    }

    fn schema() -> std::sync::Arc<MessageSchema> {
        MessageSchema::builder("ros_z::lifecycle::State")
            .field("id", FieldType::Uint8)
            .field("label", FieldType::String)
            .build()
            .expect("failed to build schema for lifecycle state")
    }

    fn schema_hash() -> SchemaHash {
        crate::dynamic::schema_hash(&Self::schema()).expect("lifecycle state schema must hash")
    }
}

impl CdrEncode for LcState {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.id.cdr_encode(w);
        self.label.cdr_encode(w);
    }
}
impl CdrDecode for LcState {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(LcState {
            id: u8::cdr_decode(r)?,
            label: String::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for LcState {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let p = self.id.cdr_encoded_size(pos);
        self.label.cdr_encoded_size(p)
    }
}

#[derive(Debug, Clone, Default)]
pub struct LcTransition {
    pub id: u8,
    pub label: String,
}

impl Message for LcTransition {
    type Codec = GeneratedCdrCodec<Self>;

    fn type_name() -> &'static str {
        "ros_z::lifecycle::Transition"
    }

    fn schema() -> std::sync::Arc<MessageSchema> {
        MessageSchema::builder("ros_z::lifecycle::Transition")
            .field("id", FieldType::Uint8)
            .field("label", FieldType::String)
            .build()
            .expect("failed to build schema for lifecycle transition")
    }

    fn schema_hash() -> SchemaHash {
        crate::dynamic::schema_hash(&Self::schema()).expect("lifecycle transition schema must hash")
    }
}

impl CdrEncode for LcTransition {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.id.cdr_encode(w);
        self.label.cdr_encode(w);
    }
}
impl CdrDecode for LcTransition {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(LcTransition {
            id: u8::cdr_decode(r)?,
            label: String::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for LcTransition {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let p = self.id.cdr_encoded_size(pos);
        self.label.cdr_encoded_size(p)
    }
}

#[derive(Debug, Clone, Default)]
pub struct LcTransitionDescription {
    pub transition: LcTransition,
    pub start_state: LcState,
    pub goal_state: LcState,
}

impl CdrEncode for LcTransitionDescription {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.transition.cdr_encode(w);
        self.start_state.cdr_encode(w);
        self.goal_state.cdr_encode(w);
    }
}
impl CdrDecode for LcTransitionDescription {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(LcTransitionDescription {
            transition: LcTransition::cdr_decode(r)?,
            start_state: LcState::cdr_decode(r)?,
            goal_state: LcState::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for LcTransitionDescription {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let p = self.transition.cdr_encoded_size(pos);
        let p = self.start_state.cdr_encoded_size(p);
        self.goal_state.cdr_encoded_size(p)
    }
}

#[derive(Debug, Clone, Default)]
pub struct LcTime {
    pub sec: i32,
    pub nanosec: u32,
}

impl CdrEncode for LcTime {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.sec.cdr_encode(w);
        self.nanosec.cdr_encode(w);
    }
}
impl CdrDecode for LcTime {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(LcTime {
            sec: i32::cdr_decode(r)?,
            nanosec: u32::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for LcTime {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let p = self.sec.cdr_encoded_size(pos);
        self.nanosec.cdr_encoded_size(p)
    }
}

#[derive(Debug, Clone, Default)]
pub struct LcTransitionEvent {
    pub timestamp: LcTime,
    pub transition: LcTransition,
    pub start_state: LcState,
    pub goal_state: LcState,
}

impl Message for LcTransitionEvent {
    type Codec = GeneratedCdrCodec<Self>;

    fn type_name() -> &'static str {
        "ros_z::lifecycle::TransitionEvent"
    }

    fn schema() -> std::sync::Arc<MessageSchema> {
        let time_schema = MessageSchema::builder("builtin_interfaces::Time")
            .field("sec", FieldType::Int32)
            .field("nanosec", FieldType::Uint32)
            .build()
            .expect("failed to build schema for builtin time");

        MessageSchema::builder("ros_z::lifecycle::TransitionEvent")
            .field("timestamp", FieldType::Message(time_schema))
            .field("transition", FieldType::Message(LcTransition::schema()))
            .field("start_state", FieldType::Message(LcState::schema()))
            .field("goal_state", FieldType::Message(LcState::schema()))
            .build()
            .expect("failed to build schema for lifecycle transition event")
    }

    fn schema_hash() -> SchemaHash {
        crate::dynamic::schema_hash(&Self::schema())
            .expect("lifecycle transition event schema must hash")
    }
}

impl CdrEncode for LcTransitionEvent {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.timestamp.cdr_encode(w);
        self.transition.cdr_encode(w);
        self.start_state.cdr_encode(w);
        self.goal_state.cdr_encode(w);
    }
}
impl CdrDecode for LcTransitionEvent {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(LcTransitionEvent {
            timestamp: LcTime::cdr_decode(r)?,
            transition: LcTransition::cdr_decode(r)?,
            start_state: LcState::cdr_decode(r)?,
            goal_state: LcState::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for LcTransitionEvent {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let p = self.timestamp.cdr_encoded_size(pos);
        let p = self.transition.cdr_encoded_size(p);
        let p = self.start_state.cdr_encoded_size(p);
        self.goal_state.cdr_encoded_size(p)
    }
}

// ---------------------------------------------------------------------------
// Service types
// ---------------------------------------------------------------------------

// --- ChangeState ---

#[derive(Debug, Clone, Default)]
pub struct ChangeStateRequest {
    pub transition: LcTransition,
}

impl CdrEncode for ChangeStateRequest {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.transition.cdr_encode(w);
    }
}
impl CdrDecode for ChangeStateRequest {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(ChangeStateRequest {
            transition: LcTransition::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for ChangeStateRequest {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        self.transition.cdr_encoded_size(pos)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChangeStateResponse {
    pub success: bool,
}

impl CdrEncode for ChangeStateResponse {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.success.cdr_encode(w);
    }
}
impl CdrDecode for ChangeStateResponse {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(ChangeStateResponse {
            success: bool::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for ChangeStateResponse {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        self.success.cdr_encoded_size(pos)
    }
}

pub struct ChangeState;

impl Service for ChangeState {
    type Request = ChangeStateRequest;
    type Response = ChangeStateResponse;
}

impl ServiceTypeInfo for ChangeState {
    fn service_type_info() -> TypeInfo {
        TypeInfo::with_hash(
            "ros_z::lifecycle::ChangeState",
            service_hash("ros_z::lifecycle::ChangeState"),
        )
    }
}

// --- GetState ---

#[derive(Debug, Clone, Default)]
pub struct GetStateRequest {}

impl CdrEncode for GetStateRequest {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, _w: &mut CdrWriter<'_, BO, B>) {}
}
impl CdrDecode for GetStateRequest {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        _r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(GetStateRequest {})
    }
}
impl CdrEncodedSize for GetStateRequest {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos
    }
}

#[derive(Debug, Clone, Default)]
pub struct GetStateResponse {
    pub current_state: LcState,
}

impl CdrEncode for GetStateResponse {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.current_state.cdr_encode(w);
    }
}
impl CdrDecode for GetStateResponse {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(GetStateResponse {
            current_state: LcState::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for GetStateResponse {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        self.current_state.cdr_encoded_size(pos)
    }
}

pub struct GetState;

impl Service for GetState {
    type Request = GetStateRequest;
    type Response = GetStateResponse;
}

impl ServiceTypeInfo for GetState {
    fn service_type_info() -> TypeInfo {
        TypeInfo::with_hash(
            "ros_z::lifecycle::GetState",
            service_hash("ros_z::lifecycle::GetState"),
        )
    }
}

// --- GetAvailableStates ---

#[derive(Debug, Clone, Default)]
pub struct GetAvailableStatesRequest {}

impl CdrEncode for GetAvailableStatesRequest {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, _w: &mut CdrWriter<'_, BO, B>) {}
}
impl CdrDecode for GetAvailableStatesRequest {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        _r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(GetAvailableStatesRequest {})
    }
}
impl CdrEncodedSize for GetAvailableStatesRequest {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos
    }
}

#[derive(Debug, Clone, Default)]
pub struct GetAvailableStatesResponse {
    pub available_states: Vec<LcState>,
}

impl CdrEncode for GetAvailableStatesResponse {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.available_states.cdr_encode(w);
    }
}
impl CdrDecode for GetAvailableStatesResponse {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(GetAvailableStatesResponse {
            available_states: Vec::<LcState>::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for GetAvailableStatesResponse {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        self.available_states.cdr_encoded_size(pos)
    }
}

pub struct GetAvailableStates;

impl Service for GetAvailableStates {
    type Request = GetAvailableStatesRequest;
    type Response = GetAvailableStatesResponse;
}

impl ServiceTypeInfo for GetAvailableStates {
    fn service_type_info() -> TypeInfo {
        TypeInfo::with_hash(
            "ros_z::lifecycle::GetAvailableStates",
            service_hash("ros_z::lifecycle::GetAvailableStates"),
        )
    }
}

// --- GetAvailableTransitions (used for both get_available_transitions and get_transition_graph) ---

#[derive(Debug, Clone, Default)]
pub struct GetAvailableTransitionsRequest {}

impl CdrEncode for GetAvailableTransitionsRequest {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, _w: &mut CdrWriter<'_, BO, B>) {}
}
impl CdrDecode for GetAvailableTransitionsRequest {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        _r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(GetAvailableTransitionsRequest {})
    }
}
impl CdrEncodedSize for GetAvailableTransitionsRequest {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos
    }
}

#[derive(Debug, Clone, Default)]
pub struct GetAvailableTransitionsResponse {
    pub available_transitions: Vec<LcTransitionDescription>,
}

impl CdrEncode for GetAvailableTransitionsResponse {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.available_transitions.cdr_encode(w);
    }
}
impl CdrDecode for GetAvailableTransitionsResponse {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(GetAvailableTransitionsResponse {
            available_transitions: Vec::<LcTransitionDescription>::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for GetAvailableTransitionsResponse {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        self.available_transitions.cdr_encoded_size(pos)
    }
}

pub struct GetAvailableTransitions;

impl Service for GetAvailableTransitions {
    type Request = GetAvailableTransitionsRequest;
    type Response = GetAvailableTransitionsResponse;
}

impl ServiceTypeInfo for GetAvailableTransitions {
    fn service_type_info() -> TypeInfo {
        TypeInfo::with_hash(
            "ros_z::lifecycle::GetAvailableTransitions",
            service_hash("ros_z::lifecycle::GetAvailableTransitions"),
        )
    }
}

fn service_hash(native_service_type: &str) -> SchemaHash {
    compute_hash(
        &ServiceDef::new(
            native_service_type,
            format!("{native_service_type}Request"),
            format!("{native_service_type}Response"),
        )
        .expect("lifecycle service descriptor must be valid"),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        ChangeState, GetAvailableStates, GetAvailableTransitions, GetState, LcState, LcTransition,
        LcTransitionEvent,
    };
    use crate::{Message, ServiceTypeInfo};

    #[test]
    fn lc_state_type_info_uses_trait_default_schema_hash() {
        let expected = crate::dynamic::schema_hash(&LcState::schema())
            .expect("LcState schema should produce a hash");

        assert_eq!(LcState::schema_hash(), expected);
    }

    #[test]
    fn lc_transition_type_info_uses_trait_default_schema_hash() {
        let expected = crate::dynamic::schema_hash(&LcTransition::schema())
            .expect("LcTransition schema should produce a hash");

        assert_eq!(LcTransition::schema_hash(), expected);
    }

    #[test]
    fn lc_transition_event_type_info_uses_trait_default_schema_hash() {
        let expected = crate::dynamic::schema_hash(&LcTransitionEvent::schema())
            .expect("LcTransitionEvent schema should produce a hash");

        assert_eq!(LcTransitionEvent::schema_hash(), expected);
    }

    #[test]
    fn lifecycle_service_type_info_uses_native_names() {
        assert_eq!(
            ChangeState::service_type_info().name,
            "ros_z::lifecycle::ChangeState"
        );
        assert_eq!(
            GetState::service_type_info().name,
            "ros_z::lifecycle::GetState"
        );
        assert_eq!(
            GetAvailableStates::service_type_info().name,
            "ros_z::lifecycle::GetAvailableStates"
        );
        assert_eq!(
            GetAvailableTransitions::service_type_info().name,
            "ros_z::lifecycle::GetAvailableTransitions"
        );
    }
}
