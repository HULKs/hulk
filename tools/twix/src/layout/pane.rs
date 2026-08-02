use std::sync::Arc;

use color_eyre::Result;
use eframe::egui::Context;
use serde::{Serialize, Serializer};
use serde_json::Value;

use crate::{
    PanelKind, SelectablePanel,
    backend::RobotBackend,
    panel::{Panel, PanelCreationContext, PanelUiContext},
    panels::TextPanel,
};

pub(super) struct PanelPane {
    pub(super) panel: SelectablePanel,
}

impl PanelPane {
    pub(super) fn restore(
        backend: &Arc<RobotBackend>,
        value: &Value,
        egui_context: &Context,
    ) -> Result<Self> {
        Ok(Self::from_panel(SelectablePanel::new(
            panel_creation_context(backend, Some(value), egui_context),
        )?))
    }

    pub(super) fn from_panel(panel: SelectablePanel) -> Self {
        Self { panel }
    }

    pub(super) fn text(backend: &Arc<RobotBackend>, egui_context: &Context) -> Self {
        Self::from_panel(SelectablePanel::TextPanel(TextPanel::new(
            panel_creation_context(backend, None, egui_context),
        )))
    }

    pub(super) fn save(&self) -> Value {
        self.panel.save()
    }

    pub(super) fn kind(&self) -> PanelKind {
        self.panel.kind()
    }
}

impl Serialize for PanelPane {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.save().serialize(serializer)
    }
}

pub(super) fn panel_creation_context<'a>(
    backend: &Arc<RobotBackend>,
    value: Option<&'a Value>,
    egui_context: &Context,
) -> PanelCreationContext<'a> {
    PanelCreationContext {
        backend: backend.clone(),
        value,
        egui_context: egui_context.clone(),
    }
}

pub(super) fn panel_ui_context<'a>(
    backend: &'a Arc<RobotBackend>,
    egui_context: &'a Context,
) -> PanelUiContext<'a> {
    PanelUiContext {
        backend,
        egui_context,
    }
}
