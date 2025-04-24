use egui::{
    vec2, KeyboardShortcut, ModifierNames, Rect, Response, RichText, Sense, Stroke,
    StrokeKind::{self},
    TextStyle, TextWrapMode, Ui, Vec2, Widget, WidgetText,
};

pub struct KeybindPreview(pub KeyboardShortcut);

fn show_key(ui: &mut Ui, key: &str, highlighted: bool) -> Response {
    ui.add(KeyPreview {
        text: key,
        highlighted,
    })
}

impl Widget for KeybindPreview {
    fn ui(self, ui: &mut Ui) -> Response {
        let names = ModifierNames::NAMES;
        let active_modifiers = ui.input(|reader| reader.modifiers);
        let is_key_pressed = ui.input(|reader| reader.key_down(self.0.logical_key));

        ui.horizontal(|ui| {
            let modifiers = self.0.modifiers;
            if modifiers.alt {
                show_key(ui, names.alt, active_modifiers.alt);
            }
            if modifiers.command || modifiers.mac_cmd {
                show_key(ui, names.mac_cmd, {
                    active_modifiers.command || active_modifiers.mac_cmd
                });
            }
            if modifiers.ctrl {
                show_key(ui, names.ctrl, active_modifiers.ctrl);
            }
            if modifiers.shift {
                show_key(ui, names.shift, active_modifiers.shift);
            }
            show_key(ui, self.0.logical_key.symbol_or_name(), is_key_pressed)
        })
        .response
    }
}

struct KeyPreview<'a> {
    text: &'a str,
    highlighted: bool,
}

impl<'a> Widget for KeyPreview<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let text = WidgetText::from(RichText::new(self.text));
        let button_padding = 4.0;
        let animation_shift_amount: f32 = 3.0;

        let galley = text.into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextStyle::Monospace,
        );

        let button_box = galley.rect.clone().expand(button_padding);
        let bounding_box = {
            let mut rect = button_box.clone();
            rect.set_height(rect.height() + animation_shift_amount);
            rect
        };
        let (response, painter) = ui.allocate_painter(bounding_box.size(), Sense::empty());
        let painter = painter.with_clip_rect(painter.clip_rect().expand(10.0));
        let amount_of_animation = ui
            .ctx()
            .animate_bool(response.id.with("key-preview"), self.highlighted);

        let text_box = Rect::from_min_size(
            response.rect.min,
            vec2(
                response.rect.width(),
                response.rect.height() - animation_shift_amount,
            ),
        );
        let animation_shift = amount_of_animation * vec2(0.0, animation_shift_amount);
        let style = ui.visuals().widgets.noninteractive;
        let highlight_bg_color = ui.visuals().widgets.hovered.bg_fill;

        painter.rect(
            text_box.translate(vec2(0.0, animation_shift_amount)),
            button_padding,
            highlight_bg_color.linear_multiply(0.1),
            Stroke::NONE,
            StrokeKind::Inside,
        );
        painter.rect(
            text_box.translate(animation_shift),
            button_padding,
            highlight_bg_color,
            style.bg_stroke,
            StrokeKind::Inside,
        );
        let text_origin = text_box.min + Vec2::splat(button_padding);
        painter.galley(text_origin + animation_shift, galley, style.text_color());
        response
    }
}
