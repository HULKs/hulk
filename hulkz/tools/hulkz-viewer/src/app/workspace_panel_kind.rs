#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspacePanelKind {
    Text,
    Parameters,
}

pub const OPENABLE_WORKSPACE_PANEL_KINDS: &[WorkspacePanelKind] =
    &[WorkspacePanelKind::Text, WorkspacePanelKind::Parameters];

impl WorkspacePanelKind {
    pub const fn label(self) -> &'static str {
        match self {
            WorkspacePanelKind::Text => "Text",
            WorkspacePanelKind::Parameters => "Parameters",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WorkspacePanelKind, OPENABLE_WORKSPACE_PANEL_KINDS};

    #[test]
    fn openable_kinds_are_workspace_only() {
        assert_eq!(OPENABLE_WORKSPACE_PANEL_KINDS.len(), 2);
        assert!(OPENABLE_WORKSPACE_PANEL_KINDS.contains(&WorkspacePanelKind::Text));
        assert!(OPENABLE_WORKSPACE_PANEL_KINDS.contains(&WorkspacePanelKind::Parameters));
    }
}
