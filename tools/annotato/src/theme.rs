use eframe::{
    egui::{self, style, Context},
    epaint::{self, Color32},
};

pub struct Theme {
    pub rosewater: Color32,
    pub flamingo: Color32,
    pub pink: Color32,
    pub mauve: Color32,
    pub red: Color32,
    pub maroon: Color32,
    pub peach: Color32,
    pub yellow: Color32,
    pub green: Color32,
    pub teal: Color32,
    pub sky: Color32,
    pub sapphire: Color32,
    pub blue: Color32,
    pub lavender: Color32,
    pub text: Color32,
    pub subtext1: Color32,
    pub subtext0: Color32,
    pub overlay2: Color32,
    pub overlay1: Color32,
    pub overlay0: Color32,
    pub surface2: Color32,
    pub surface1: Color32,
    pub surface0: Color32,
    pub base: Color32,
    pub mantle: Color32,
    pub crust: Color32,
}

pub const MOCHA: Theme = Theme {
    rosewater: Color32::from_rgb(245, 224, 220),
    flamingo: Color32::from_rgb(242, 205, 205),
    pink: Color32::from_rgb(245, 194, 231),
    mauve: Color32::from_rgb(203, 166, 247),
    red: Color32::from_rgb(243, 139, 168),
    maroon: Color32::from_rgb(235, 160, 172),
    peach: Color32::from_rgb(250, 179, 135),
    yellow: Color32::from_rgb(249, 226, 175),
    green: Color32::from_rgb(166, 227, 161),
    teal: Color32::from_rgb(148, 226, 213),
    sky: Color32::from_rgb(137, 220, 235),
    sapphire: Color32::from_rgb(116, 199, 236),
    blue: Color32::from_rgb(137, 180, 250),
    lavender: Color32::from_rgb(180, 190, 254),
    text: Color32::from_rgb(205, 214, 244),
    subtext1: Color32::from_rgb(186, 194, 222),
    subtext0: Color32::from_rgb(166, 173, 200),
    overlay2: Color32::from_rgb(147, 153, 178),
    overlay1: Color32::from_rgb(127, 132, 156),
    overlay0: Color32::from_rgb(108, 112, 134),
    surface2: Color32::from_rgb(88, 91, 112),
    surface1: Color32::from_rgb(69, 71, 90),
    surface0: Color32::from_rgb(49, 50, 68),
    base: Color32::from_rgb(30, 30, 46),
    mantle: Color32::from_rgb(24, 24, 37),
    crust: Color32::from_rgb(17, 17, 27),
};

pub fn apply_theme(ctx: &Context, theme: Theme) {
    let old = ctx.style().visuals.clone();
    ctx.set_visuals(egui::Visuals {
        override_text_color: Some(theme.text),
        hyperlink_color: theme.rosewater,
        faint_bg_color: theme.surface0,
        extreme_bg_color: theme.crust,
        code_bg_color: theme.mantle,
        warn_fg_color: theme.peach,
        error_fg_color: theme.maroon,
        window_fill: theme.base,
        panel_fill: theme.base,
        window_stroke: egui::Stroke {
            color: theme.overlay1,
            ..old.window_stroke
        },
        widgets: style::Widgets {
            noninteractive: make_widget_visual(old.widgets.noninteractive, &theme, theme.base),
            inactive: make_widget_visual(old.widgets.inactive, &theme, theme.surface0),
            hovered: make_widget_visual(old.widgets.hovered, &theme, theme.surface2),
            active: make_widget_visual(old.widgets.active, &theme, theme.surface1),
            open: make_widget_visual(old.widgets.open, &theme, theme.surface0),
        },
        selection: style::Selection {
            bg_fill: theme.blue.linear_multiply(0.2),
            stroke: egui::Stroke {
                color: theme.overlay1,
                ..old.selection.stroke
            },
        },
        window_shadow: epaint::Shadow {
            color: theme.base,
            ..old.window_shadow
        },
        popup_shadow: epaint::Shadow {
            color: theme.base,
            ..old.popup_shadow
        },
        ..old
    });
}

fn make_widget_visual(
    old: style::WidgetVisuals,
    theme: &Theme,
    bg_fill: egui::Color32,
) -> style::WidgetVisuals {
    style::WidgetVisuals {
        bg_fill,
        weak_bg_fill: bg_fill,
        bg_stroke: egui::Stroke {
            color: theme.overlay1,
            ..old.bg_stroke
        },
        fg_stroke: egui::Stroke {
            color: theme.text,
            ..old.fg_stroke
        },
        ..old
    }
}
