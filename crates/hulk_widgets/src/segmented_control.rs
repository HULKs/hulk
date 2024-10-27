use egui::{
    vec2, Align2, Context, Id, InnerResponse, Key, Rect, Response, Rounding, Sense, TextStyle, Ui,
    Widget,
};

pub struct SegmentedControl<'ui, T> {
    selectables: &'ui [T],
    id: Id,
    rounding: Option<Rounding>,
    text_style: TextStyle,
}

#[derive(Debug, Default, Clone)]
struct SegmentedControlState {
    selected: usize,
}

impl<'ui, T: ToString> SegmentedControl<'ui, T> {
    pub fn new(id: impl Into<Id>, selectables: &'ui [T]) -> Self {
        SegmentedControl {
            id: id.into(),
            selectables,
            rounding: None,
            text_style: TextStyle::Body,
        }
    }

    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.rounding = Some(rounding.into());
        self
    }

    pub fn ui(self, ui: &mut Ui) -> InnerResponse<&'ui T> {
        let mut state = load_state(ui.ctx(), self.id);
        let response = self.show(ui, &mut state);
        let selected = &self.selectables[state.selected];
        save_state(ui.ctx(), self.id, state);
        InnerResponse::new(selected, response)
    }

    fn show(&self, ui: &mut Ui, state: &mut SegmentedControlState) -> Response {
        let width = ui.available_width();
        let text_style = ui
            .style()
            .text_styles
            .get(&self.text_style)
            .expect("failed to get text style")
            .clone();
        let text_size = text_style.size * ui.ctx().pixels_per_point();
        let rounding = self
            .rounding
            .unwrap_or(ui.style().noninteractive().rounding);

        let (mut response, painter) =
            ui.allocate_painter(vec2(width, 2.0 * text_size), Sense::hover());
        if response.contains_pointer() {
            ui.input(|reader| {
                if reader.key_pressed(Key::ArrowLeft) {
                    state.selected = state.selected.saturating_sub(1);
                    response.mark_changed();
                } else if reader.key_pressed(Key::ArrowRight) {
                    state.selected = (state.selected + 1).min(self.selectables.len() - 1);
                    response.mark_changed();
                }
            })
        }
        painter.rect_filled(response.rect, rounding, ui.style().visuals.extreme_bg_color);

        let text_rects = text_rects(response.rect, self.selectables.len());
        let offset = text_rects[0].width();

        let translation = animate_to(self.id, ui.ctx(), offset * state.selected as f32);
        let selector_rect = text_rects[0].translate(vec2(translation, 0.0)).shrink(2.0);
        let selector_response =
            ui.interact(selector_rect, self.id.with("selector"), Sense::click());
        let selector_style = ui.style().interact(&selector_response);
        painter.rect_filled(selector_rect, rounding, selector_style.bg_fill);

        let noninteractive_style = ui.style().noninteractive();

        for (idx, (&rect, text)) in text_rects.iter().zip(self.selectables.iter()).enumerate() {
            let label_response = ui.interact(rect, self.id.with(idx), Sense::click());
            let style = ui.style().interact(&response);

            let show_line = idx > 0 && state.selected != idx && state.selected + 1 != idx;
            {
                let animated_height = ui
                    .ctx()
                    .animate_bool(self.id.with("vline").with(idx), show_line);

                let height = vec2(0.0, rect.height() - 4.0);
                let center = rect.left_center();

                painter.line_segment(
                    [
                        center - 0.5 * animated_height * height,
                        center + 0.5 * animated_height * height,
                    ],
                    noninteractive_style.bg_stroke,
                );
            }

            if label_response.clicked() {
                state.selected = idx;
                response.mark_changed();
            }
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                text.to_string(),
                text_style.clone(),
                style.text_color(),
            );
        }
        response
    }
}

impl<'ui, T: ToString> Widget for SegmentedControl<'ui, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut state = load_state(ui.ctx(), self.id);
        let response = self.show(ui, &mut state);
        save_state(ui.ctx(), self.id, state);
        response
    }
}

fn load_state(ctx: &Context, id: Id) -> SegmentedControlState {
    let persisted = ctx.data_mut(|reader| reader.get_temp(id));
    persisted.unwrap_or_default()
}

fn save_state(ctx: &Context, id: Id, state: SegmentedControlState) {
    ctx.data_mut(|writer| writer.insert_temp(id, state));
}

fn animate_to(source: Id, context: &Context, target: f32) -> f32 {
    context.animate_value_with_time(source, target, 0.1)
}

fn text_rects(mut rect: Rect, number_of_texts: usize) -> Vec<Rect> {
    let base_width = rect.width() / number_of_texts as f32;
    let base_rect = {
        rect.set_width(base_width);
        rect
    };
    (0..number_of_texts)
        .map(|idx| {
            let rect = base_rect;
            rect.translate(vec2(base_width * idx as f32, 0.0))
        })
        .collect()
}
