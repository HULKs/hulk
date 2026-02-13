#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum PanelKind {
    Text,
    Parameters,
}

pub(super) const OPENABLE_PANEL_KINDS: &[PanelKind] = &[PanelKind::Text, PanelKind::Parameters];

impl PanelKind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            PanelKind::Text => "Text",
            PanelKind::Parameters => "Parameters",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PanelKind, OPENABLE_PANEL_KINDS};

    #[test]
    fn openable_kinds_are_workspace_only() {
        assert_eq!(OPENABLE_PANEL_KINDS.len(), 2);
        assert!(OPENABLE_PANEL_KINDS.contains(&PanelKind::Text));
        assert!(OPENABLE_PANEL_KINDS.contains(&PanelKind::Parameters));
    }
}
