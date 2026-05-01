use std::marker::PhantomData;

use ros_z_cdr::{CdrBuffer, CdrDecode, CdrEncode, CdrEncodedSize, CdrReader, CdrWriter};
use serde::{Deserialize, Serialize};

use super::{Action, GoalId, GoalInfo, GoalStatus};

// Native ros-z cancel control messages.

/// Request to cancel one or more goals.
///
/// Used to request cancellation of specific goals or all goals.
/// A zero UUID in `goal_info.goal_id` cancels all goals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelGoalRequest {
    /// Information about the goal(s) to cancel.
    pub goal_info: GoalInfo,
}

/// Response to a cancel goal request.
///
/// Contains the result code and list of goals that are being canceled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelGoalResponse {
    /// Return code indicating success or failure.
    pub return_code: i8,
    /// List of goals that are being canceled.
    pub goals_canceling: Vec<GoalInfo>,
}

// Internal service/topic message types

/// Request to send a goal to an action server.
///
/// Contains the goal ID and the actual goal data.
#[derive(Debug, Clone)]
pub struct GoalRequest<A: Action> {
    /// Unique identifier for this goal.
    pub goal_id: GoalId,
    /// The goal data to be executed.
    pub goal: A::Goal,
}

impl<A: Action> serde::Serialize for GoalRequest<A>
where
    A: 'static,
    A::Goal: serde::Serialize + 'static,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("GoalRequest", 2)?;
        state.serialize_field("goal_id", &self.goal_id)?;
        state.serialize_field("goal", &self.goal)?;
        state.end()
    }
}

impl<'de, A: Action> serde::Deserialize<'de> for GoalRequest<A>
where
    A: 'static,
    A::Goal: serde::Deserialize<'de> + 'static,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct GoalRequestHelper<B> {
            goal_id: GoalId,
            goal: B,
        }
        let helper = GoalRequestHelper::<A::Goal>::deserialize(deserializer)?;
        Ok(GoalRequest {
            goal_id: helper.goal_id,
            goal: helper.goal,
        })
    }
}

/// Response to a goal request.
///
/// Indicates whether the goal was accepted and includes a timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalResponse {
    /// Whether the goal was accepted by the server.
    pub accepted: bool,
    /// Timestamp seconds (corresponds to builtin_interfaces/Time.sec)
    pub stamp_sec: i32,
    /// Timestamp nanoseconds (corresponds to builtin_interfaces/Time.nanosec)
    pub stamp_nanosec: u32,
}

/// Request to get the result of a completed goal.
///
/// Contains the goal ID for which to retrieve the result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultRequest {
    /// The ID of the goal whose result is requested.
    pub goal_id: GoalId,
}

/// Response containing the result of a completed goal.
///
/// Includes the final status and the result data.
#[derive(Debug, Clone)]
pub struct ResultResponse<A: Action> {
    /// The final status of the goal.
    pub status: GoalStatus,
    /// The result data returned by the action server.
    pub result: A::Result,
}

impl<A: Action> serde::Serialize for ResultResponse<A>
where
    A: 'static,
    A::Result: serde::Serialize + 'static,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ResultResponse", 2)?;
        state.serialize_field("status", &self.status)?;
        state.serialize_field("result", &self.result)?;
        state.end()
    }
}

impl<'de, A: Action> serde::Deserialize<'de> for ResultResponse<A>
where
    A: 'static,
    A::Result: serde::Deserialize<'de> + 'static,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct ResultResponseHelper<B> {
            status: GoalStatus,
            result: B,
        }
        let helper = ResultResponseHelper::<A::Result>::deserialize(deserializer)?;
        Ok(ResultResponse {
            status: helper.status,
            result: helper.result,
        })
    }
}

/// Message containing feedback from an executing goal.
///
/// Feedback messages are published periodically during goal execution
/// to provide progress updates to clients.
///
/// Note: This type does NOT implement Message because the schema hash
/// is action-specific and must be provided via A::feedback_type_info()
#[derive(Debug, Clone)]
pub struct FeedbackMessage<A: Action> {
    /// The ID of the goal providing feedback.
    pub goal_id: GoalId,
    /// The feedback data from the executing goal.
    pub feedback: A::Feedback,
}

impl<A: Action> serde::Serialize for FeedbackMessage<A>
where
    A: 'static,
    A::Feedback: serde::Serialize + 'static,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("FeedbackMessage", 2)?;
        state.serialize_field("goal_id", &self.goal_id)?;
        state.serialize_field("feedback", &self.feedback)?;
        state.end()
    }
}

impl<'de, A: Action> serde::Deserialize<'de> for FeedbackMessage<A>
where
    A: 'static,
    A::Feedback: serde::Deserialize<'de> + 'static,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct FeedbackMessageHelper<B> {
            goal_id: GoalId,
            feedback: B,
        }
        let helper = FeedbackMessageHelper::<A::Feedback>::deserialize(deserializer)?;
        Ok(FeedbackMessage {
            goal_id: helper.goal_id,
            feedback: helper.feedback,
        })
    }
}

/// Message containing status updates for multiple goals.
///
/// Published periodically to inform clients about the current status
/// of all goals known to the action server.
///
/// Note: This type does NOT implement Message because the schema hash
/// is action-specific and must be provided via A::status_type_info()
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMessage {
    /// List of status information for all goals.
    pub status_list: Vec<GoalStatusInfo>,
}

/// Status information for a single goal.
///
/// Contains the goal info and current status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalStatusInfo {
    /// Information about the goal (ID and timestamp).
    pub goal_info: GoalInfo,
    /// Current status of the goal.
    pub status: GoalStatus,
}

// Internal service type wrappers
/// Native SendGoal service request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendGoalRequest<A: Action> {
    pub goal_id: GoalId,
    pub goal: A::Goal,
}

/// Native SendGoal service response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendGoalResponse {
    pub accepted: bool,
    pub stamp_sec: i32,
    pub stamp_nanosec: u32,
}

pub struct GoalService<A: Action>(PhantomData<A>);
impl<A: Action> crate::msg::Service for GoalService<A> {
    type Request = SendGoalRequest<A>;
    type Response = SendGoalResponse;
}

impl<A: Action> crate::ServiceTypeInfo for GoalService<A> {
    fn service_type_info() -> crate::entity::TypeInfo {
        // Delegate to the action's send_goal_type_info method.
        A::send_goal_type_info()
    }
}

/// Native GetResult service request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetResultRequest {
    pub goal_id: GoalId,
}

/// Native GetResult service response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetResultResponse<A: Action> {
    pub status: i8,
    pub result: A::Result,
}

pub struct ResultService<A: Action>(PhantomData<A>);
impl<A: Action> crate::msg::Service for ResultService<A> {
    type Request = GetResultRequest;
    type Response = GetResultResponse<A>;
}

impl<A: Action> crate::ServiceTypeInfo for ResultService<A> {
    fn service_type_info() -> crate::entity::TypeInfo {
        // Delegate to the action's get_result_type_info method.
        A::get_result_type_info()
    }
}

/// Native CancelGoal service request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelGoalServiceRequest {
    pub goal_info: GoalInfo,
}

/// Native CancelGoal service response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelGoalServiceResponse {
    pub return_code: i8,
    pub goals_canceling: Vec<GoalInfo>,
}

pub struct CancelService<A: Action>(PhantomData<A>);
impl<A: Action> crate::msg::Service for CancelService<A> {
    type Request = CancelGoalServiceRequest;
    type Response = CancelGoalServiceResponse;
}

impl<A: Action> crate::ServiceTypeInfo for CancelService<A> {
    fn service_type_info() -> crate::entity::TypeInfo {
        // Delegate to the action's cancel_goal_type_info method.
        A::cancel_goal_type_info()
    }
}

// ── CDR serialization impls ───────────────────────────────────────────────────
// Concrete (non-generic) action message types now implement CdrEncode +
// CdrDecode + CdrEncodedSize directly, so the blanket
// `impl<T: CdrEncode + ...> WireMessage for T` covers them automatically.
//
// Generic types (GoalRequest<A>, etc.) still use the serde path via explicit
// WireMessage impls below until Action's associated type bounds are updated.

impl CdrEncode for GoalStatusInfo {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.goal_info.cdr_encode(w);
        self.status.cdr_encode(w);
    }
}
impl CdrDecode for GoalStatusInfo {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(GoalStatusInfo {
            goal_info: GoalInfo::cdr_decode(r)?,
            status: GoalStatus::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for GoalStatusInfo {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let p = self.goal_info.cdr_encoded_size(pos);
        self.status.cdr_encoded_size(p)
    }
}

impl CdrEncode for CancelGoalRequest {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.goal_info.cdr_encode(w);
    }
}
impl CdrDecode for CancelGoalRequest {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(CancelGoalRequest {
            goal_info: GoalInfo::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for CancelGoalRequest {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        self.goal_info.cdr_encoded_size(pos)
    }
}

impl CdrEncode for CancelGoalResponse {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.return_code.cdr_encode(w);
        self.goals_canceling.cdr_encode(w);
    }
}
impl CdrDecode for CancelGoalResponse {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(CancelGoalResponse {
            return_code: i8::cdr_decode(r)?,
            goals_canceling: Vec::<GoalInfo>::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for CancelGoalResponse {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let p = self.return_code.cdr_encoded_size(pos);
        self.goals_canceling.cdr_encoded_size(p)
    }
}

impl CdrEncode for GoalResponse {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.accepted.cdr_encode(w);
        self.stamp_sec.cdr_encode(w);
        self.stamp_nanosec.cdr_encode(w);
    }
}
impl CdrDecode for GoalResponse {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(GoalResponse {
            accepted: bool::cdr_decode(r)?,
            stamp_sec: i32::cdr_decode(r)?,
            stamp_nanosec: u32::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for GoalResponse {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let p = self.accepted.cdr_encoded_size(pos);
        let p = self.stamp_sec.cdr_encoded_size(p);
        self.stamp_nanosec.cdr_encoded_size(p)
    }
}

impl CdrEncode for ResultRequest {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.goal_id.cdr_encode(w);
    }
}
impl CdrDecode for ResultRequest {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(ResultRequest {
            goal_id: GoalId::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for ResultRequest {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        self.goal_id.cdr_encoded_size(pos)
    }
}

impl CdrEncode for StatusMessage {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.status_list.cdr_encode(w);
    }
}
impl CdrDecode for StatusMessage {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(StatusMessage {
            status_list: Vec::<GoalStatusInfo>::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for StatusMessage {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        self.status_list.cdr_encoded_size(pos)
    }
}

impl CdrEncode for SendGoalResponse {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.accepted.cdr_encode(w);
        self.stamp_sec.cdr_encode(w);
        self.stamp_nanosec.cdr_encode(w);
    }
}
impl CdrDecode for SendGoalResponse {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(SendGoalResponse {
            accepted: bool::cdr_decode(r)?,
            stamp_sec: i32::cdr_decode(r)?,
            stamp_nanosec: u32::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for SendGoalResponse {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let p = self.accepted.cdr_encoded_size(pos);
        let p = self.stamp_sec.cdr_encoded_size(p);
        self.stamp_nanosec.cdr_encoded_size(p)
    }
}

impl CdrEncode for GetResultRequest {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.goal_id.cdr_encode(w);
    }
}
impl CdrDecode for GetResultRequest {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(GetResultRequest {
            goal_id: GoalId::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for GetResultRequest {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        self.goal_id.cdr_encoded_size(pos)
    }
}

impl CdrEncode for CancelGoalServiceRequest {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.goal_info.cdr_encode(w);
    }
}
impl CdrDecode for CancelGoalServiceRequest {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(CancelGoalServiceRequest {
            goal_info: GoalInfo::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for CancelGoalServiceRequest {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        self.goal_info.cdr_encoded_size(pos)
    }
}

impl CdrEncode for CancelGoalServiceResponse {
    fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        self.return_code.cdr_encode(w);
        self.goals_canceling.cdr_encode(w);
    }
}
impl CdrDecode for CancelGoalServiceResponse {
    fn cdr_decode<'de, BO: byteorder::ByteOrder>(
        r: &mut CdrReader<'de, BO>,
    ) -> ros_z_cdr::Result<Self> {
        Ok(CancelGoalServiceResponse {
            return_code: i8::cdr_decode(r)?,
            goals_canceling: Vec::<GoalInfo>::cdr_decode(r)?,
        })
    }
}
impl CdrEncodedSize for CancelGoalServiceResponse {
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let p = self.return_code.cdr_encoded_size(pos);
        self.goals_canceling.cdr_encoded_size(p)
    }
}

// ── Generic types: still use serde path until Action gains CDR bounds ────────

impl<A: Action + 'static> crate::msg::WireMessage for GoalRequest<A>
where
    A::Goal: Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
{
    type Codec = crate::msg::SerdeCdrCodec<GoalRequest<A>>;
}

impl<A: Action + 'static> crate::msg::WireMessage for ResultResponse<A>
where
    A::Result: Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
{
    type Codec = crate::msg::SerdeCdrCodec<ResultResponse<A>>;
}

impl<A: Action + 'static> crate::msg::WireMessage for FeedbackMessage<A>
where
    A::Feedback: Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
{
    type Codec = crate::msg::SerdeCdrCodec<FeedbackMessage<A>>;
}

impl<A: Action + 'static> crate::msg::WireMessage for SendGoalRequest<A>
where
    A::Goal: Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
{
    type Codec = crate::msg::SerdeCdrCodec<SendGoalRequest<A>>;
}

impl<A: Action + 'static> crate::msg::WireMessage for GetResultResponse<A>
where
    A::Result: Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
{
    type Codec = crate::msg::SerdeCdrCodec<GetResultResponse<A>>;
}
