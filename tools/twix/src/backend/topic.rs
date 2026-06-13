use color_eyre::{Result, eyre::eyre};
use ros_z::topic_name::qualify_topic_name;

const TWIX_TARGET_NODE_PLACEHOLDER: &str = "twix_target";

pub fn normalize_namespace(namespace: &str) -> Result<String> {
    let trimmed = namespace.trim_matches('/');
    let normalized = if trimmed.is_empty() {
        "/".to_string()
    } else {
        format!("/{trimmed}")
    };

    qualify_topic_name(
        "twix_namespace_probe",
        &normalized,
        TWIX_TARGET_NODE_PLACEHOLDER,
    )
    .map_err(|source| eyre!("invalid namespace '{namespace}': {source}"))?;

    Ok(normalized)
}

#[cfg(test)]
pub fn resolve_topic_selector(target_namespace: &str, selector: &str) -> Result<String> {
    if selector.starts_with('~') {
        return Err(eyre!(
            "private topic selectors are unsupported in Twix: '{selector}'"
        ));
    }

    let namespace = if selector.starts_with('/') {
        "/".to_string()
    } else {
        normalize_namespace(target_namespace)?
    };

    qualify_topic_name(selector, &namespace, TWIX_TARGET_NODE_PLACEHOLDER)
        .map_err(|source| eyre!("invalid topic selector '{selector}': {source}"))
}

pub fn display_selector(target_namespace: &str, resolved_topic: &str) -> Result<String> {
    let target_namespace = normalize_namespace(target_namespace)?;
    if !resolved_topic.starts_with('/') {
        return Err(eyre!("resolved topic must be absolute: '{resolved_topic}'"));
    }

    let resolved_topic = qualify_topic_name(resolved_topic, "/", TWIX_TARGET_NODE_PLACEHOLDER)
        .map_err(|source| eyre!("invalid resolved topic '{resolved_topic}': {source}"))?;

    if target_namespace == "/" {
        return Ok(resolved_topic);
    }

    let prefix = format!("{target_namespace}/");
    match resolved_topic.strip_prefix(&prefix) {
        Some(relative_topic) => Ok(relative_topic.to_string()),
        None => Ok(resolved_topic),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_bare_robot_namespace() {
        assert_eq!(normalize_namespace("42").unwrap(), "/42");
        assert_eq!(normalize_namespace("/42").unwrap(), "/42");
        assert_eq!(normalize_namespace("/").unwrap(), "/");
    }

    #[test]
    fn rejects_invalid_namespace() {
        let error = normalize_namespace("robot%01").unwrap_err();
        assert!(error.to_string().contains("invalid namespace"));
    }

    #[test]
    fn resolves_relative_selector_against_namespace() {
        assert_eq!(
            resolve_topic_selector("42", "ground_to_field").unwrap(),
            "/42/ground_to_field"
        );
        assert_eq!(
            resolve_topic_selector("/robot-01", "behavior/trace").unwrap(),
            "/robot-01/behavior/trace"
        );
    }

    #[test]
    fn leaves_absolute_selector_absolute() {
        assert_eq!(
            resolve_topic_selector("42", "/diagnostics").unwrap(),
            "/diagnostics"
        );
    }

    #[test]
    fn rejects_private_selector() {
        let error = resolve_topic_selector("42", "~private").unwrap_err();
        assert!(error.to_string().contains("private topic selectors"));
    }

    #[test]
    fn displays_topics_relative_to_selected_namespace() {
        assert_eq!(
            display_selector("/42", "/42/ground_to_field").unwrap(),
            "ground_to_field"
        );
        assert_eq!(
            display_selector("/42", "/diagnostics").unwrap(),
            "/diagnostics"
        );
    }

    #[test]
    fn preserves_absolute_topics_under_root_namespace() {
        assert_eq!(
            display_selector("/", "/diagnostics").unwrap(),
            "/diagnostics"
        );
    }

    #[test]
    fn rejects_relative_resolved_topic_input() {
        let error = display_selector("/42", "ground_to_field").unwrap_err();
        assert!(error.to_string().contains("resolved topic"));
    }

    #[test]
    fn relative_selector_changes_when_namespace_changes() {
        assert_ne!(
            resolve_topic_selector("/42", "ground_to_field").unwrap(),
            resolve_topic_selector("/43", "ground_to_field").unwrap()
        );
    }

    #[test]
    fn absolute_selector_survives_namespace_changes() {
        assert_eq!(
            resolve_topic_selector("/42", "/diagnostics").unwrap(),
            resolve_topic_selector("/43", "/diagnostics").unwrap()
        );
    }
}
