//! Structured editors for the exact published message fields, using native Bevy widgets.
mod choices;

use std::collections::HashSet;

use bevy::{
    camera_controller::free_camera::FreeCameraState,
    feathers::{
        FeathersPlugins, controls::*, dark_theme::create_dark_theme, display::label, theme::UiTheme,
    },
    input_focus::{InputFocus, tab_navigation::TabGroup},
    prelude::*,
    ui_widgets::{Activate, ValueChange},
};
use coordinate_systems::Ground;
use linear_algebra::Orientation2;
use serde_json::{Value, json};
use types::{
    filtered_game_controller_state::FilteredGameControllerState, motion_command::MotionCommand,
};

use crate::{
    bevy_mujoco::{MujocoWorld, SimulationMode},
    robotics::Robotics,
    simulation::SimulationControl,
};
use choices::{choices, is_angle, value, variant};

pub const PANEL_WIDTH: f32 = 460.0;

#[derive(Resource)]
struct Editor {
    draft: Value,
    tab: &'static str,
    rebuild: bool,
    message: String,
    expanded: HashSet<String>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            draft: json!({"motion": value(MotionCommand::Damping), "game": value(FilteredGameControllerState::default())}),
            tab: "motion",
            rebuild: true,
            expanded: HashSet::new(),
            message: "Edit a command, then press Send. Numbers support dragging and text entry."
                .into(),
        }
    }
}

#[derive(Component)]
struct Form;
#[derive(Component)]
struct Status;
#[derive(Component)]
struct SimulationStatus;

pub struct ControlsPlugin;
impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FeathersPlugins)
            .insert_resource(UiTheme(create_dark_theme()))
            .init_resource::<Editor>()
            .add_systems(Startup, setup)
            .add_systems(PreUpdate, gate_camera_input)
            .add_systems(Update, (rebuild_form, update_status));
    }
}

fn gate_camera_input(
    window: Single<&Window>,
    focus: Res<InputFocus>,
    editors: Query<(), With<bevy::text::EditableText>>,
    mut camera: Single<&mut FreeCameraState>,
) {
    let over_scene = window
        .cursor_position()
        .is_some_and(|position| position.x >= 240.0 && position.x < window.width() - PANEL_WIDTH);
    camera.enabled = over_scene && !focus.get().is_some_and(|entity| editors.contains(entity));
    if !camera.enabled {
        camera.velocity = Vec3::ZERO;
    }
}

fn column(commands: &mut Commands, parent: Entity) -> Entity {
    commands
        .spawn((
            ChildOf(parent),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                width: percent(100),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id()
}
fn text(commands: &mut Commands, parent: Entity, content: impl Into<String>) {
    commands.spawn((
        ChildOf(parent),
        Text::new(content),
        TextFont::from_font_size(14.0),
        TextColor(Color::srgb(0.85, 0.89, 0.95)),
    ));
}

fn setup(mut commands: Commands) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                top: px(0),
                bottom: px(0),
                width: px(PANEL_WIDTH),
                padding: UiRect::all(px(14)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                ..default()
            },
            BackgroundColor(Color::srgb(0.055, 0.065, 0.08)),
            GlobalZIndex(10),
            TabGroup::default(),
        ))
        .id();
    text(&mut commands, root, "MOTION LAB");
    commands.spawn((
        ChildOf(root),
        SimulationStatus,
        Text::new("Paused"),
        TextFont::from_font_size(13.0),
    ));
    let toolbar = commands
        .spawn((
            ChildOf(root),
            Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(6),
                row_gap: px(6),
                ..default()
            },
        ))
        .id();
    commands.spawn_scene(bsn! {
        @FeathersButton ChildOf(toolbar) Children[label("Run / Pause")]
        on(|_: On<Activate>, mut mode: ResMut<SimulationMode>| {
            *mode = match *mode { SimulationMode::Paused => SimulationMode::Running, SimulationMode::Running => SimulationMode::Paused };
        })
    });
    commands.spawn_scene(bsn! {
        @FeathersButton ChildOf(toolbar) Children[label("Reset robot & stack")]
        on(|_: On<Activate>, mut control: ResMut<SimulationControl>| { control.reset = true; })
    });
    commands.spawn_scene(bsn! {
        @FeathersButton ChildOf(toolbar) Children[label("Send Damping")]
        on(|_: On<Activate>, mut io: ResMut<Robotics>, mut editor: ResMut<Editor>| {
            io.input_motion = MotionCommand::Damping;
            editor.draft["motion"] = value(MotionCommand::Damping);
            editor.rebuild = true;
            editor.message = report(io.publish_inputs(), "Head damping requested; body remains at zero pose.");
        })
    });
    let tabs = commands
        .spawn((
            ChildOf(root),
            Node {
                column_gap: px(6),
                ..default()
            },
        ))
        .id();
    for (name, title) in [("motion", "Motion command"), ("game", "Game controller")] {
        commands.spawn_scene(bsn! {
            @FeathersButton ChildOf(tabs) Children[label(title)]
            on(move |_: On<Activate>, mut editor: ResMut<Editor>| { editor.tab = name; editor.rebuild = true; })
        });
    }
    commands.spawn_scene(bsn! {
        @FeathersButton ChildOf(root) Children[label("Send current form")]
        on(|_: On<Activate>, mut editor: ResMut<Editor>, mut io: ResMut<Robotics>| {
            let result = if editor.tab == "motion" {
                serde_json::from_value::<MotionCommand>(editor.draft["motion"].clone()).map(|command| io.input_motion = command)
            } else {
                serde_json::from_value::<FilteredGameControllerState>(editor.draft["game"].clone()).map(|game| io.input_game = game)
            };
            editor.message = match result {
                Ok(()) => report(io.publish_inputs(), if editor.tab == "motion" { "Head request published; body remains at zero pose." } else { "Game controller state published." }),
                Err(error) => format!("Cannot send: {error}"),
            };
        })
    });
    commands.spawn((
        ChildOf(root),
        Status,
        Text::new(""),
        TextFont::from_font_size(13.0),
    ));
    // PointerScroll bubbles to the form even when a child widget is under the cursor.
    commands
        .spawn((
            ChildOf(root),
            Form,
            Node {
                overflow: Overflow::scroll_y(),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                ..default()
            },
        ))
        .observe(
            |mut event: On<PointerScroll>,
             mut forms: Query<(&mut ScrollPosition, &ComputedNode), With<Form>>| {
                if let Ok((mut scroll, computed)) = forms.get_mut(event.event_target()) {
                    let max = (computed.content_size().y - computed.size().y)
                        * computed.inverse_scale_factor();
                    scroll.y = (scroll.y
                        - event.y
                            * if event.unit == bevy::input::mouse::MouseScrollUnit::Line {
                                30.0
                            } else {
                                1.0
                            })
                    .clamp(0.0, max.max(0.0));
                    event.propagate(false);
                }
            },
        );
}

fn report(result: color_eyre::Result<()>, success: &str) -> String {
    result.map_or_else(
        |e| format!("Publish failed: {e:#}"),
        |()| success.to_owned(),
    )
}

fn update_status(
    editor: Res<Editor>,
    io: Res<Robotics>,
    world: Res<MujocoWorld>,
    mode: Res<SimulationMode>,
    mut status: Single<&mut Text, (With<Status>, Without<SimulationStatus>)>,
    mut simulation: Single<&mut Text, (With<SimulationStatus>, Without<Status>)>,
) {
    status.0 = editor.message.clone();
    simulation.0 = format!(
        "{:?}  |  {:.3} s\n{}\nJoint commands: {}",
        *mode,
        world.data().time(),
        io.status(),
        if io.latest_command().is_some() {
            "receiving"
        } else {
            "waiting"
        }
    );
}

fn rebuild_form(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    form: Single<Entity, With<Form>>,
    children: Query<&Children>,
) {
    if !editor.rebuild {
        return;
    }
    editor.rebuild = false;
    if let Ok(children) = children.get(*form) {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }
    let path = format!("/{}", editor.tab);
    let title = if editor.tab == "motion" {
        "MotionCommand"
    } else {
        "FilteredGameControllerState"
    };
    build_field(
        &mut commands,
        *form,
        &path,
        title,
        &editor.draft[editor.tab],
        true,
        &editor.expanded,
    );
}

fn choice_button(
    commands: &mut Commands,
    parent: Entity,
    path: &str,
    option: Value,
    selected: bool,
) {
    let name = format!("{}{}", if selected { "[x] " } else { "" }, variant(&option));
    let path = path.to_owned();
    commands.spawn_scene(bsn! {
        @FeathersButton ChildOf(parent) Children[label(&name)]
        on(move |_: On<Activate>, mut editor: ResMut<Editor>| {
            if let Some(slot) = editor.draft.pointer_mut(&path) { *slot = option.clone(); }
            editor.rebuild = true;
        })
    });
}

fn build_field(
    commands: &mut Commands,
    parent: Entity,
    path: &str,
    title: &str,
    current: &Value,
    allow_choices: bool,
    expanded: &HashSet<String>,
) {
    let group = column(commands, parent);
    let title = title.replace('_', " ");
    text(commands, group, &title);
    if path.ends_with("/penalties") || path.ends_with("_penalties_last_cycle") {
        let open = expanded.contains(path);
        let key = path.to_owned();
        commands.spawn_scene(bsn! {
            @FeathersButton ChildOf(group) Children[label(if open { "Hide player penalties" } else { "Edit player penalties" })]
            on(move |_: On<Activate>, mut editor: ResMut<Editor>| {
                if !editor.expanded.remove(&key) { editor.expanded.insert(key.clone()); }
                editor.rebuild = true;
            })
        });
        if !open {
            return;
        }
    }
    if allow_choices && let Some(options) = choices(path) {
        let row = commands
            .spawn((
                ChildOf(group),
                Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(4),
                    row_gap: px(4),
                    ..default()
                },
            ))
            .id();
        for option in options {
            choice_button(
                commands,
                row,
                path,
                option.clone(),
                variant(&option) == variant(current),
            );
        }
        if let Value::Object(fields) = current {
            for (key, value) in fields {
                build_field(
                    commands,
                    group,
                    &format!("{path}/{key}"),
                    "Fields",
                    value,
                    false,
                    expanded,
                );
            }
        }
        return;
    }
    if is_angle(path, current) {
        let angle = serde_json::from_value::<Orientation2<Ground>>(current.clone())
            .unwrap()
            .angle();
        number(commands, group, path, angle as f64, Numeric::Angle);
        return;
    }
    if let Value::Object(fields) = current {
        if fields.contains_key("secs") && fields.contains_key("nanos") && fields.len() == 2 {
            let duration: std::time::Duration = serde_json::from_value(current.clone()).unwrap();
            number(
                commands,
                group,
                path,
                duration.as_secs_f64(),
                Numeric::Duration,
            );
        } else if path.ends_with("_penalties_last_cycle") {
            // Map entries are optional; absence means no new penalty for that player this cycle.
            for player in ["One", "Two", "Three", "Four", "Five"] {
                let key = format!("{path}/{player}");
                let row = column(commands, group);
                text(commands, row, format!("Player {player}"));
                for option in choices::penalty_choices() {
                    let path = path.to_owned();
                    let option_label = variant(&option).to_owned();
                    commands.spawn_scene(bsn! {
                        @FeathersButton ChildOf(row) Children[label(&option_label)]
                        on(move |_: On<Activate>, mut editor: ResMut<Editor>| {
                            let map = editor.draft.pointer_mut(&path).unwrap().as_object_mut().unwrap();
                            if option.is_null() { map.remove(player); } else { map.insert(player.to_owned(), option.clone()); }
                            editor.rebuild = true;
                        })
                    });
                }
                if let Some(value) = fields.get(player) {
                    build_field(
                        commands,
                        row,
                        &key,
                        "Current penalty",
                        value,
                        false,
                        expanded,
                    );
                }
            }
        } else {
            for (key, value) in fields {
                build_field(
                    commands,
                    group,
                    &format!("{path}/{key}"),
                    &field_label(key),
                    value,
                    true,
                    expanded,
                );
            }
        }
    } else if let Value::Array(values) = current {
        for (index, value) in values.iter().enumerate() {
            let item_label = if path.ends_with("/segments") {
                format!("Segment {}", index + 1)
            } else if path.ends_with("/LineSegment") {
                ["Start (Ground, m)", "End (Ground, m)"][index].to_owned()
            } else {
                ["x", "y", "z"]
                    .get(index)
                    .map_or_else(|| index.to_string(), |s| (*s).to_owned())
            };
            build_field(
                commands,
                group,
                &format!("{path}/{index}"),
                &item_label,
                value,
                true,
                expanded,
            );
            if path.ends_with("/segments") {
                let path = path.to_owned();
                commands.spawn_scene(bsn! {
                    @FeathersButton ChildOf(group) Children[label("Remove segment")]
                    on(move |_: On<Activate>, mut editor: ResMut<Editor>| {
                        editor.draft.pointer_mut(&path).unwrap().as_array_mut().unwrap().remove(index);
                        editor.rebuild = true;
                    })
                });
            }
        }
        if path.ends_with("/segments") {
            for segment in choices::segment_choices() {
                let label_text = format!("Add {}", variant(&segment));
                let path = path.to_owned();
                commands.spawn_scene(bsn! {
                    @FeathersButton ChildOf(group) Children[label(&label_text)]
                    on(move |_: On<Activate>, mut editor: ResMut<Editor>| {
                        editor.draft.pointer_mut(&path).unwrap().as_array_mut().unwrap().push(segment.clone());
                        editor.rebuild = true;
                    })
                });
            }
        }
    } else if let Value::Number(n) = current {
        number(
            commands,
            group,
            path,
            n.as_f64().unwrap(),
            if n.is_u64() {
                Numeric::Unsigned
            } else {
                Numeric::Float
            },
        );
    } else if let Value::Bool(b) = current {
        let path = path.to_owned();
        let next = !b;
        commands.spawn_scene(bsn! {
            @FeathersButton ChildOf(group) Children[label(if *b { "[x] Enabled" } else { "Disabled" })]
            on(move |_: On<Activate>, mut editor: ResMut<Editor>| {
                *editor.draft.pointer_mut(&path).unwrap() = Value::Bool(next);
                editor.rebuild = true;
            })
        });
    } else {
        text(commands, group, variant(current));
    }
}

#[derive(Clone, Copy)]
enum Numeric {
    Float,
    Unsigned,
    Angle,
    Duration,
}
fn number(commands: &mut Commands, parent: Entity, path: &str, initial: f64, kind: Numeric) {
    let path = path.to_owned();
    let units = match kind {
        Numeric::Angle => "rad",
        Numeric::Duration => "s",
        _ => "",
    };
    text(commands, parent, units);
    commands.spawn_scene(bsn! {
        @FeathersNumberInput
        ChildOf(parent)
        template_value(NumberInputValue::F64(initial))
        NumberInputPrecision(4)
        NumberInputStep(0.01)
        Node { width: percent(100), min_height: px(28) }
        on(move |event: On<ValueChange<f64>>, mut commands: Commands, mut editor: ResMut<Editor>| {
            let n = event.value;
            if !n.is_finite() || n.abs() > f32::MAX as f64 || matches!(kind, Numeric::Unsigned | Numeric::Duration) && n < 0.0 { return; }
            let next = match kind {
                Numeric::Float => json!(n),
                Numeric::Unsigned => json!(n.round() as u64),
                Numeric::Angle => value(Orientation2::<Ground>::new(n as f32)),
                Numeric::Duration => match std::time::Duration::try_from_secs_f64(n) { Ok(d) => value(d), Err(_) => return },
            };
            *editor.draft.pointer_mut(&path).unwrap() = next;
            commands.entity(event.event_target()).insert(NumberInputValue::F64(n));
        })
    });
}

fn field_label(name: &str) -> String {
    match name {
        "velocity" => "velocity (Ground, m/s)",
        "angular_velocity" => "angular velocity (rad/s)",
        "ball_position" | "target_position" | "target" | "center" => {
            return format!("{} (Ground, m)", name.replace('_', " "));
        }
        "robot_theta_to_field" => "robot theta to field (Field, rad)",
        "kick_direction" | "target_orientation" | "direction" | "tolerance" => {
            return format!("{} (rad)", name.replace('_', " "));
        }
        "distance_to_be_aligned" | "radius" => return format!("{} (m)", name.replace('_', " ")),
        _ => name,
    }
    .replace('_', " ")
}
