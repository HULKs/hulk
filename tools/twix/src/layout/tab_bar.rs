use eframe::egui::{
    Align2, Button, CornerRadius, FontId, Id, Popup, PopupCloseBehavior, Rect, Response, RichText,
    Sense, StrokeKind, TextStyle, Ui, vec2,
};
use egui_material_icons::icons;
use egui_tiles::{Behavior as _, TabState, TileId, Tiles};
use hulk_widgets::SearchableSelector;

use crate::{PanelKind, SelectablePanel};

use super::{
    TREE_ID,
    behavior::LayoutBehavior,
    focus::request_pane_focus,
    tree::{LayoutRequest, active_pane_in_tile},
};

pub(super) fn tab_title(tiles: &Tiles<SelectablePanel>, tile_id: TileId) -> String {
    tiles
        .get_pane(&tile_id)
        .map_or_else(|| "Group".to_string(), |pane| pane.kind().label())
}

pub(super) fn tab_ui(
    behavior: &mut LayoutBehavior<'_>,
    tiles: &mut Tiles<SelectablePanel>,
    ui: &mut Ui,
    id: Id,
    tile_id: TileId,
    state: &TabState,
) -> Response {
    let current = tiles.get_pane(&tile_id).map(SelectablePanel::kind);
    let title = behavior.tab_title_for_tile(tiles, tile_id);
    let font_id = TextStyle::Button.resolve(ui.style());
    let galley = title.into_galley(
        ui,
        Some(eframe::egui::TextWrapMode::Extend),
        f32::INFINITY,
        font_id,
    );
    let x_margin = behavior.tab_title_spacing(ui.visuals());
    let accessory_size = vec2(
        if current.is_some() { 20.0 } else { 0.0 },
        ui.available_height(),
    );
    let close_size = eframe::egui::Vec2::splat(behavior.close_button_outer_size());
    let close_width = if state.closable {
        close_size.x + 4.0
    } else {
        0.0
    };
    let width = 2.0 * x_margin + galley.size().x + accessory_size.x + close_width;
    let (_, tab_rect) = ui.allocate_space(vec2(width, ui.available_height()));
    let tab_response = ui
        .interact(tab_rect, id, Sense::click_and_drag())
        .on_hover_cursor(eframe::egui::CursorIcon::Grab);
    if tab_response.drag_started() {
        close_layout_popups(ui, tiles);
    }

    let right = tab_rect.right() - x_margin;
    let close_rect = Rect::from_center_size(
        eframe::egui::pos2(right - close_size.x / 2.0, tab_rect.center().y),
        close_size,
    );
    let accessory_right = right - close_width;
    let accessory_rect = Rect::from_min_max(
        eframe::egui::pos2(accessory_right - accessory_size.x, tab_rect.top()),
        eframe::egui::pos2(accessory_right, tab_rect.bottom()),
    );
    let selector_response = current.map(|_| {
        ui.interact(accessory_rect, id.with("panel-type"), Sense::click())
            .on_hover_cursor(eframe::egui::CursorIcon::PointingHand)
            .on_hover_text("Change panel type")
    });

    if !state.is_being_dragged && ui.is_rect_visible(tab_rect) {
        let background = behavior.tab_bg_color(ui.visuals(), tiles, tile_id, state);
        let stroke = behavior.tab_outline_stroke(ui.visuals(), tiles, tile_id, state);
        ui.painter().rect(
            tab_rect.shrink(0.5),
            CornerRadius::ZERO,
            background,
            stroke,
            StrokeKind::Inside,
        );
        if state.active {
            ui.painter().hline(
                tab_rect.x_range(),
                tab_rect.bottom(),
                (stroke.width + 1.0, background),
            );
        }

        let text_color = behavior.tab_text_color(ui.visuals(), tiles, tile_id, state);
        let text_rect = Rect::from_min_max(
            tab_rect.min + vec2(x_margin, 0.0),
            eframe::egui::pos2(accessory_rect.left(), tab_rect.bottom()),
        );
        let text_position = Align2::LEFT_CENTER
            .align_size_within_rect(galley.size(), text_rect)
            .min;
        ui.painter().galley(text_position, galley, text_color);

        if let Some(selector_response) = &selector_response {
            let selector_visuals = ui.style().interact(selector_response);
            if selector_response.hovered() || selector_response.has_focus() {
                ui.painter().rect_filled(
                    accessory_rect.shrink(2.0),
                    2,
                    selector_visuals.weak_bg_fill,
                );
            }
            ui.painter().text(
                accessory_rect.center(),
                Align2::CENTER_CENTER,
                icons::ICON_KEYBOARD_ARROW_DOWN.codepoint,
                FontId::proportional(12.0),
                selector_visuals.text_color(),
            );
        }

        if state.closable {
            let close_response = ui
                .interact(close_rect, id.with("close"), Sense::click())
                .on_hover_cursor(eframe::egui::CursorIcon::PointingHand)
                .on_hover_text("Close tab");
            let visuals = ui.style().interact(&close_response);
            let rect = close_rect
                .shrink(behavior.close_button_inner_margin())
                .expand(visuals.expansion);
            ui.painter()
                .line_segment([rect.left_top(), rect.right_bottom()], visuals.fg_stroke);
            ui.painter()
                .line_segment([rect.right_top(), rect.left_bottom()], visuals.fg_stroke);
            if close_response.clicked() {
                behavior.requests.push(LayoutRequest::Close(tile_id));
            }
        }
    }

    let selector_clicked = selector_response.as_ref().is_some_and(Response::clicked);
    if (tab_response.clicked() || selector_clicked || tab_response.drag_started())
        && let Some(pane_id) = active_pane_in_tile(tiles, tile_id)
    {
        *behavior.focused = Some(pane_id);
        request_pane_focus(&behavior.egui_context, pane_id);
    }
    if tab_response.middle_clicked() {
        behavior.requests.push(LayoutRequest::Close(tile_id));
    }

    if let Some(selector_response) = &selector_response {
        let popup_id = panel_type_popup_id(tile_id);
        let popup_was_open = Popup::is_id_open(ui.ctx(), popup_id);
        let mut reset_picker = false;
        if *behavior.selector_to_open == Some(tile_id) {
            Popup::open_id(ui.ctx(), popup_id);
            *behavior.selector_to_open = None;
            reset_picker = true;
        } else if selector_response.clicked() {
            Popup::toggle_id(ui.ctx(), popup_id);
            reset_picker = !popup_was_open;
        }

        let replacement = Popup::from_response(selector_response)
            .id(popup_id)
            .open_memory(None)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .width(220.0)
            .show(|ui| {
                panel_picker(
                    ui,
                    id.with("panel-picker"),
                    current,
                    "Panel type",
                    reset_picker,
                )
            })
            .and_then(|response| response.inner);
        if let Some(panel) = replacement
            && current != Some(panel)
        {
            behavior.requests.push(LayoutRequest::Replace {
                pane: tile_id,
                panel,
            });
        }
    }

    match selector_response {
        Some(selector_response) => tab_response | selector_response,
        None => tab_response,
    }
}

pub(super) fn add_panel_button(behavior: &mut LayoutBehavior<'_>, ui: &mut Ui, tabs_id: TileId) {
    let add_response = ui
        .add(
            Button::new(icons::ICON_ADD.codepoint)
                .frame(false)
                .min_size(vec2(24.0, 24.0)),
        )
        .on_hover_text("Add panel");
    let popup_id = add_panel_popup_id(tabs_id);
    let reset_picker = add_response.clicked() && !Popup::is_id_open(ui.ctx(), popup_id);
    let selected = Popup::from_toggle_button_response(&add_response)
        .id(popup_id)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .width(220.0)
        .show(|ui| {
            panel_picker(
                ui,
                ui.id().with("panel-picker"),
                None,
                "New panel",
                reset_picker,
            )
        })
        .and_then(|response| response.inner);
    if let Some(panel) = selected {
        behavior.requests.push(LayoutRequest::Add {
            tabs: tabs_id,
            panel,
        });
    }
}

fn close_layout_popups(ui: &Ui, tiles: &Tiles<SelectablePanel>) {
    for tile_id in tiles.tile_ids() {
        Popup::close_id(ui.ctx(), panel_type_popup_id(tile_id));
        Popup::close_id(ui.ctx(), add_panel_popup_id(tile_id));
    }
}

pub(super) fn panel_type_popup_id(tile_id: TileId) -> Id {
    tile_id.egui_id(Id::new(TREE_ID)).with("panel-type-popup")
}

fn add_panel_popup_id(tabs_id: TileId) -> Id {
    Id::new((TREE_ID, tabs_id, "add-panel-popup"))
}

fn panel_picker(
    ui: &mut Ui,
    id: Id,
    current: Option<PanelKind>,
    title: &str,
    reset_on_show: bool,
) -> Option<PanelKind> {
    ui.set_min_width(220.0);
    ui.strong(title);
    ui.add_space(2.0);

    let descriptors = PanelKind::registered();
    let selected = SearchableSelector::new(id, descriptors)
        .reset_on_show(reset_on_show)
        .show(
            ui,
            |panel| panel.display_name(),
            |ui, highlighted, panel| {
                let is_current = current == Some(*panel);
                ui.add(
                    Button::selectable(highlighted, panel.label())
                        .right_text(RichText::new(if is_current { "Current" } else { "" }).weak())
                        .min_size(vec2(ui.available_width(), 28.0)),
                )
            },
        );
    if selected.is_some() {
        ui.close();
    }
    selected.and_then(|index| descriptors.get(index)).copied()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use eframe::egui::{CentralPanel, Context, RawInput};

    use crate::{backend::RobotBackend, layout::TwixLayout};

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn new_tab_reveal_settles_for_fitting_and_oversized_tabs() {
        let backend = Arc::new(
            RobotBackend::new(tokio::runtime::Handle::current(), None, "/".into())
                .await
                .unwrap(),
        );
        for width in [140.0, 240.0, 640.0] {
            let context = Context::default();
            let mut layout = TwixLayout::new(&context, &backend);
            for _ in 0..10 {
                layout.open_tab(&backend, &context);
            }
            // Check both first-frame overflow and insertion into an established tab bar.
            for frame in 0..10 {
                if frame == 5 {
                    layout.open_tab(&backend, &context);
                    assert_eq!(layout.tab_to_reveal, layout.focused);
                }
                let input = RawInput {
                    screen_rect: Some(Rect::from_min_size(Default::default(), vec2(width, 200.0))),
                    ..Default::default()
                };
                let _ = context.run_ui(input, |ui| {
                    CentralPanel::default().show(ui, |ui| layout.ui(ui, &backend));
                });
                if frame == 4 || frame == 9 {
                    let response = context
                        .read_response(layout.focused.unwrap().egui_id(Id::new(TREE_ID)))
                        .unwrap();
                    assert_eq!(layout.tab_to_reveal, None);
                    assert!(response.interact_rect.is_positive());
                    assert_eq!(
                        response.interact_rect.contains_rect(response.rect),
                        width >= 240.0,
                        "width={width} frame={frame} {response:?}"
                    );
                }
            }
        }
    }
}
