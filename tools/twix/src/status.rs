use ros_z_debug::{
    CachedSubscriptionStatus, CachedSubscriptionStatusSnapshot, TopicObservationBlockReason,
    TopicObservationStatus,
};

pub fn format_topic_observation_status(status: TopicObservationStatus) -> String {
    match status {
        TopicObservationStatus::Building => "building".to_string(),
        TopicObservationStatus::Observing { cache } => {
            format!("observing: {}", format_cache_status(&cache))
        }
        TopicObservationStatus::Rebuilding { previous_cache } => {
            format!(
                "rebuilding: previous {}",
                format_cache_status(&previous_cache)
            )
        }
        TopicObservationStatus::Retrying {
            previous_cache,
            error,
        } => format_with_optional_previous("retrying", &error, previous_cache),
        TopicObservationStatus::Blocked {
            previous_cache,
            reason,
        } => {
            format_with_optional_previous("blocked", &format_block_reason(&reason), previous_cache)
        }
        TopicObservationStatus::Closed => "closed".to_string(),
        _ => "unknown".to_string(),
    }
}

fn format_with_optional_previous(
    state: &str,
    message: &str,
    previous_cache: Option<CachedSubscriptionStatusSnapshot>,
) -> String {
    match previous_cache {
        Some(previous_cache) => {
            format!(
                "{state}: {message}; previous {}",
                format_cache_status(&previous_cache)
            )
        }
        None => format!("{state}: {message}"),
    }
}

fn format_cache_status(cache: &CachedSubscriptionStatusSnapshot) -> String {
    let mut parts = vec![format_subscription_status(cache.status()).to_string()];

    if let Some(topic) = cache.resolved_topic() {
        parts.push(format!("topic={topic}"));
    }

    if let Some(type_info) = cache.type_info() {
        parts.push(format!("type={}", type_info.name));
        parts.push(format!("hash={:#}", type_info.hash));
    }

    if let Some(message) = cache.message() {
        parts.push(format!("message={message}"));
    }

    parts.join(", ")
}

fn format_subscription_status(status: &CachedSubscriptionStatus) -> &'static str {
    match status {
        CachedSubscriptionStatus::WaitingForFirstSample => "waiting for first sample",
        CachedSubscriptionStatus::Ready => "ready",
        CachedSubscriptionStatus::ProtocolError { .. } => "protocol error",
        CachedSubscriptionStatus::DecodeError { .. } => "decode error",
        CachedSubscriptionStatus::Closed => "closed",
        _ => "unknown",
    }
}

fn format_block_reason(reason: &TopicObservationBlockReason) -> String {
    match reason {
        TopicObservationBlockReason::MissingTargetNodeName { topic } => {
            format!("missing target node for {topic}")
        }
        _ => "unknown block reason".to_string(),
    }
}
