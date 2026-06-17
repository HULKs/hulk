use color_eyre::{Result, eyre::eyre};
use ros_z_debug::{ManagerOptions, TopicProjection};

pub fn normalize_namespace(namespace: &str) -> Result<String> {
    Ok(ManagerOptions::with_target_namespace(namespace)?
        .target_namespace()
        .to_string())
}

#[cfg(test)]
pub fn resolve_topic_selector(target_namespace: &str, selector: &str) -> Result<String> {
    Ok(ros_z_debug::TopicSelector::new(selector)?.resolve(target_namespace)?)
}

#[cfg(test)]
pub fn display_selector(target_namespace: &str, resolved_topic: &str) -> Result<String> {
    if !resolved_topic.starts_with('/') {
        return Err(eyre!("resolved topic must be absolute: '{resolved_topic}'"));
    }

    let projected = TopicProjection::project(target_namespace, [resolved_topic])?;
    Ok(projected
        .into_iter()
        .next()
        .ok_or_else(|| eyre!("topic projection returned no topics"))?
        .display_name)
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
        assert!(error.to_string().contains("private topic selector"));
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
    fn displays_root_namespace_topics_relative_to_root() {
        assert_eq!(
            display_selector("/", "/diagnostics").unwrap(),
            "diagnostics"
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
