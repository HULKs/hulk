use egui::{vec2, Align2, CornerRadius, Id, Key, Rect, Response, Sense, TextStyle, Ui, Widget};

const ANIMATION_TIME_SECONDS: f32 = 0.1;

pub struct SegmentedControl<'ui, T> {
    id: Id,
    selected: &'ui mut usize,
    selectables: &'ui [T],
    corner_radius: Option<CornerRadius>,
    text_style: TextStyle,
}

impl<'ui, T: ToString> SegmentedControl<'ui, T> {
    pub fn new(id: impl Into<Id>, selected: &'ui mut usize, selectables: &'ui [T]) -> Self {
        SegmentedControl {
            id: id.into(),
            selected,
            selectables,
            corner_radius: None,
            text_style: TextStyle::Body,
        }
    }

    pub fn corner_radius(mut self, corner_radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius = Some(corner_radius.into());
        self
    }
}

impl<T: ToString> Widget for SegmentedControl<'_, T> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let this = &mut self;
        let width = ui.available_width();
        let text_style = ui
            .style()
            .text_styles
            .get(&this.text_style)
            .expect("failed to get text style")
            .clone();
        let text_size = text_style.size * ui.ctx().pixels_per_point();
        let corner_radius = this
            .corner_radius
            .unwrap_or(ui.style().noninteractive().corner_radius);

        let (mut response, painter) =
            ui.allocate_painter(vec2(width, 2.0 * text_size), Sense::hover());
        if response.contains_pointer() {
            ui.input(|reader| {
                if reader.key_pressed(Key::ArrowLeft) || reader.key_pressed(Key::ArrowDown) {
                    *this.selected = this.selected.saturating_sub(1);
                    response.mark_changed();
                } else if reader.key_pressed(Key::ArrowRight) || reader.key_pressed(Key::ArrowUp) {
                    *this.selected = (*this.selected + 1).min(this.selectables.len() - 1);
                    response.mark_changed();
                }
            })
        }
        painter.rect_filled(
            response.rect,
            corner_radius,
            ui.style().visuals.extreme_bg_color,
        );

        let text_rects = text_rects(response.rect, this.selectables.len());
        let offset = text_rects[0].width();

        let translation = ui.ctx().animate_value_with_time(
            this.id,
            offset * *this.selected as f32,
            ANIMATION_TIME_SECONDS,
        );
        let selector_rect = text_rects[0].translate(vec2(translation, 0.0)).shrink(2.0);
        let selector_response =
            ui.interact(selector_rect, this.id.with("selector"), Sense::click());
        let selector_style = ui.style().interact(&selector_response);
        painter.rect_filled(selector_rect, corner_radius, selector_style.bg_fill);

        let noninteractive_style = ui.style().noninteractive();

        for (idx, (&rect, text)) in text_rects.iter().zip(this.selectables.iter()).enumerate() {
            let label_response = ui.interact(rect, this.id.with(idx), Sense::click());
            let style = ui.style().interact(&response);

            let show_line = idx > 0 && *this.selected != idx && *this.selected + 1 != idx;
            {
                let animated_height = ui
                    .ctx()
                    .animate_bool(this.id.with("vline").with(idx), show_line);

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
                *this.selected = idx;
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
