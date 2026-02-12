#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum PanelKind {
    Discovery,
    Timeline,
    Text,
    Parameters,
}

pub(super) const OPENABLE_PANEL_KINDS: &[PanelKind] = &[PanelKind::Text, PanelKind::Parameters];

impl PanelKind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            PanelKind::Discovery => "Discovery",
            PanelKind::Timeline => "Timeline",
            PanelKind::Text => "Text",
            PanelKind::Parameters => "Parameters",
        }
    }

    pub(super) const fn is_singleton(self) -> bool {
        matches!(self, PanelKind::Discovery | PanelKind::Timeline)
    }

    pub(super) const fn is_fixed(self) -> bool {
        matches!(self, PanelKind::Discovery | PanelKind::Timeline)
    }
}
