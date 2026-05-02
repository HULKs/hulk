use crate::entity::TypeInfo;

/// Trait for ROS service types that provides service-level type information.
///
/// For services, the type name should be based on the service name (not Request/Response)
/// and the hash should be the composite service hash (not just request or response hash).
///
/// The service hash in ROS2 is computed from a composite type that includes:
/// - request_message (the Request type)
/// - response_message (the Response type)
/// - event_message (a virtual Event type containing ServiceEventInfo, request[], and response[])
///
pub trait ServiceTypeInfo {
    /// Returns the service type info (type name and hash for the service).
    fn service_type_info() -> TypeInfo;
}
