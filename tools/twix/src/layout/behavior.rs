use std::sync::Arc;

use eframe::egui::{
    Button, Context, CornerRadius, Id, Margin, Rect, Response, Sense, StrokeKind, Ui, UiBuilder,
    WidgetInfo, WidgetText, WidgetType, vec2,
};
use egui_tiles::{
    Behavior, EditAction, SimplificationOptions, TabState, TileId, Tiles, UiResponse,
};

use crate::{PanelKind, backend::RobotBackend};

use super::{
    TREE_ID,
    focus::pane_focus_id,
    pane::{PanelPane, panel_ui_context},
    tab_bar,
    tree::{LayoutRequest, TabGroupId, simplification_options},
};

pub(super) struct LayoutBehavior<'a> {
    pub(super) backend: &'a Arc<RobotBackend>,
    pub(super) egui_context: Context,
    pub(super) focused: &'a mut Option<TileId>,
    pub(super) tab_to_reveal: &'a mut Option<TileId>,
    pub(super) selector_to_open: &'a mut Option<TileId>,
    pub(super) focus_dirty: &'a mut bool,
    pub(super) requests: Vec<LayoutRequest>,
}

impl Behavior<PanelPane> for LayoutBehavior<'_> {
    fn pane_ui(&mut self, ui: &mut Ui, tile_id: TileId, pane: &mut PanelPane) -> UiResponse {
        let clicked_in_pane =
            ui.rect_contains_pointer(ui.max_rect()) && ui.input(|input| input.pointer.any_click());
        let kind = pane.kind();
        let pane = ui.scope_builder(
            UiBuilder::new()
                .id(pane_focus_id(tile_id))
                .max_rect(ui.max_rect())
                .sense(Sense::focusable_noninteractive()),
            |ui| {
                ui.expand_to_include_rect(ui.max_rect());
                if clicked_in_pane {
                    ui.response().request_focus();
                }
                eframe::egui::Frame::new()
                    .inner_margin(Margin::same(4))
                    .show(ui, |ui| {
                        let header_fill = ui.visuals().faint_bg_color;
                        eframe::egui::Frame::new()
                            .fill(header_fill)
                            .inner_margin(Margin::symmetric(6, 4))
                            .corner_radius(3)
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    pane.panel.header_ui(
                                        ui,
                                        panel_ui_context(self.backend, &self.egui_context),
                                    );
                                });
                            });
                        ui.add_space(4.0);
                        pane.panel
                            .ui(ui, panel_ui_context(self.backend, &self.egui_context));
                    });
            },
        );
        pane.response
            .widget_info(|| WidgetInfo::labeled(WidgetType::Panel, true, kind.display_name()));
        if clicked_in_pane || pane.response.gained_focus() {
            *self.focused = Some(tile_id);
        }

        UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &PanelPane) -> WidgetText {
        pane.kind().label().into()
    }

    fn tab_title_for_tile(&mut self, tiles: &Tiles<PanelPane>, tile_id: TileId) -> WidgetText {
        tab_bar::tab_title(tiles, tile_id).into()
    }

    fn tab_ui(
        &mut self,
        tiles: &mut Tiles<PanelPane>,
        ui: &mut Ui,
        id: eframe::egui::Id,
        tile_id: TileId,
        state: &TabState,
    ) -> Response {
        tab_bar::tab_ui(self, tiles, ui, id, tile_id, state)
    }

    fn top_bar_right_ui(
        &mut self,
        _tiles: &Tiles<PanelPane>,
        ui: &mut Ui,
        tile_id: TileId,
        tabs: &egui_tiles::Tabs,
        scroll_offset: &mut f32,
    ) {
        if let Some(tab) = *self.tab_to_reveal
            && tabs.children.contains(&tab)
            && let Some(response) = self
                .egui_context
                .read_response(tab.egui_id(Id::new(TREE_ID)))
        {
            if !response.interact_rect.contains_rect(response.rect) {
                *scroll_offset = f32::INFINITY;
            }
            *self.tab_to_reveal = None;
        }
        if ui
            .add(
                Button::new(egui_material_icons::icons::ICON_CLOSE.codepoint)
                    .frame(false)
                    .min_size(vec2(24.0, 24.0)),
            )
            .on_hover_text("Close all tabs in this group")
            .clicked()
        {
            self.requests.push(LayoutRequest::Close(tile_id));
        }
    }

    fn tab_bar_trailing_ui(
        &mut self,
        _tiles: &Tiles<PanelPane>,
        ui: &mut Ui,
        tile_id: TileId,
        _tabs: &egui_tiles::Tabs,
    ) {
        if ui.ctx().dragged_id().is_none() && ui.ctx().drag_stopped_id().is_none() {
            tab_bar::add_panel_button(self, ui, tile_id);
        }
    }

    fn is_tab_closable(&self, tiles: &Tiles<PanelPane>, tile_id: TileId) -> bool {
        tiles.get(tile_id).is_some()
    }

    fn simplification_options(&self) -> SimplificationOptions {
        simplification_options()
    }

    fn tab_bar_height(&self, _style: &eframe::egui::Style) -> f32 {
        28.0
    }

    fn paint_on_top_of_tile(
        &self,
        painter: &eframe::egui::Painter,
        style: &eframe::egui::Style,
        tile_id: TileId,
        rect: Rect,
    ) {
        if *self.focused == Some(tile_id) {
            painter.rect_stroke(
                rect.shrink(0.5),
                CornerRadius::same(3),
                style.visuals.widgets.active.bg_stroke,
                StrokeKind::Inside,
            );
        }
    }

    fn on_edit(&mut self, edit_action: EditAction) {
        if edit_action == EditAction::TileDropped {
            *self.focus_dirty = true;
        }
        self.egui_context.request_repaint();
    }
}

impl LayoutBehavior<'_> {
    pub(super) fn request_add(&mut self, tabs: TileId, panel: PanelKind) {
        self.requests.push(LayoutRequest::Add {
            tabs: TabGroupId(tabs),
            panel,
        });
    }
}
