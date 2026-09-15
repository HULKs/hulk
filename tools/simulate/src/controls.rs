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
use linear_algebra::{Orientation2, point};
use serde_json::{Value, json};
use types::{
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
};

use crate::{
    bevy_mujoco::{MujocoWorld, SimulationMode},
    motion_parameters::MotionParameters,
    robot_io::RobotBinding,
    robotics::Robotics,
    scene::ball::SpawnedBalls,
    simulation::{ControlledRobot, SimulationControl},
};
use choices::{choices, is_angle, value, variant};

pub const PANEL_WIDTH: f32 = 460.0;

fn look_at_first_ball(
    world: &MujocoWorld,
    balls: &SpawnedBalls,
    robot: Entity,
) -> color_eyre::Result<MotionCommand> {
    let ball = balls.0.first().ok_or_else(|| {
        color_eyre::eyre::eyre!("No ball in the scene. Drag a ball onto the field first.")
    })?;
    let data = world.data();
    let ball = data
        .body(&format!("object_{}_ball", ball.to_bits()))
        .ok_or_else(|| color_eyre::eyre::eyre!("The first ball is not ready in MuJoCo yet."))?
        .view(data);
    let robot = RobotBinding::new(data, &format!("object_{}_", robot.to_bits()))?;
    let target = robot.point_in_ground(data, [ball.xpos[0], ball.xpos[1], ball.xpos[2]]);
    Ok(MotionCommand::Stand {
        head: HeadMotion::LookAt {
            target: point![target.x, target.y],
            height_above_ground: target.z,
            image_region_target: ImageRegion::Center,
        },
    })
}

#[derive(Resource)]
struct Editor {
    draft: Value,
    tab: &'static str,
    rebuild: bool,
    message: String,
    expanded: HashSet<String>,
    rendered_tab: &'static str,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            draft: json!({"motion": value(MotionCommand::Damping), "game": value(FilteredGameControllerState::default())}),
            tab: "motion",
            rebuild: true,
            expanded: HashSet::new(),
            rendered_tab: "",
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
#[derive(Component, Default, Clone)]
struct SendForm;
#[derive(Component, Clone)]
struct ParameterText(String);

pub struct ControlsPlugin;
impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FeathersPlugins)
            .insert_resource(UiTheme(create_dark_theme()))
            .init_resource::<Editor>()
            .add_systems(Startup, setup)
            .add_systems(PreUpdate, gate_camera_input)
            .add_systems(
                Update,
                (sync_parameter_text, rebuild_form, update_status).chain(),
            );
    }
}

fn gate_camera_input(
    window: Single<&Window>,
    focus: Res<InputFocus>,
    editors: Query<(), With<bevy::text::EditableText>>,
    mut camera: Single<&mut FreeCameraState>,
    balls: Res<crate::scene::ball_interaction::BallSelection>,
) {
    let over_scene = window
        .cursor_position()
        .is_some_and(|position| position.x >= 240.0 && position.x < window.width() - PANEL_WIDTH);
    camera.enabled = over_scene
        && !balls.is_dragging()
        && !focus.get().is_some_and(|entity| editors.contains(entity));
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

fn setup(mut commands: Commands, mut editor: ResMut<Editor>, io: Res<Robotics>) {
    match io.parameter_values() {
        Ok(parameters) => editor.draft["parameters"] = parameters,
        Err(error) => editor.message = format!("Could not load motion parameters: {error:#}"),
    }
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
            editor.message = report(io.publish_inputs(), "Head damping requested; zero-velocity walking remains active.");
        })
    });
    commands.spawn_scene(bsn! {
        @FeathersButton ChildOf(toolbar) Children[label("Look at ball")]
        on(|_: On<Activate>, world: Res<MujocoWorld>, balls: Res<SpawnedBalls>,
            robot: Single<Entity, With<ControlledRobot>>, mut io: ResMut<Robotics>, mut editor: ResMut<Editor>| {
            match look_at_first_ball(&world, &balls, *robot) {
                Ok(command) => {
                    editor.draft["motion"] = value(&command);
                    editor.tab = "motion";
                    editor.rebuild = true;
                    io.input_motion = command;
                    editor.message = report(io.publish_inputs(), "LookAt sent for the first ball's current position.");
                }
                Err(error) => editor.message = error.to_string(),
            }
        })
    });
    let tabs = commands
        .spawn((
            ChildOf(root),
            Node {
                column_gap: px(6),
                flex_wrap: FlexWrap::Wrap,
                row_gap: px(6),
                ..default()
            },
        ))
        .id();
    for (name, title) in [
        ("motion", "Motion command"),
        ("game", "Game controller"),
        ("parameters", "Parameters"),
    ] {
        commands.spawn_scene(bsn! {
            @FeathersButton ChildOf(tabs) Children[label(title)]
            on(move |_: On<Activate>, mut editor: ResMut<Editor>| {
                editor.tab = name;
                editor.rebuild = true;
                editor.message = if name == "parameters" {
                    "Expand a parameter group to edit it, then apply the complete form."
                } else {
                    "Edit a command, then press Send. Numbers support dragging and text entry."
                }.into();
            })
        });
    }
    commands.spawn_scene(bsn! {
        @FeathersButton ChildOf(root) SendForm Children[label("Send current form")]
        on(|_: On<Activate>, mut editor: ResMut<Editor>, mut io: ResMut<Robotics>| {
            if editor.tab == "parameters" {
                editor.message = "Use Apply parameters & reset below to apply this form.".into();
                return;
            }
            let result = if editor.tab == "motion" {
                serde_json::from_value::<MotionCommand>(editor.draft["motion"].clone()).map(|command| io.input_motion = command)
            } else {
                serde_json::from_value::<FilteredGameControllerState>(editor.draft["game"].clone()).map(|game| io.input_game = game)
            };
            editor.message = match result {
                Ok(()) => report(io.publish_inputs(), if editor.tab == "motion" { "Head request published; zero-velocity walking remains active." } else { "Game controller state published." }),
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
    mut editor: ResMut<Editor>,
    mut control: ResMut<SimulationControl>,
    io: Res<Robotics>,
    world: Res<MujocoWorld>,
    mode: Res<SimulationMode>,
    mut status: Single<&mut Text, (With<Status>, Without<SimulationStatus>)>,
    mut simulation: Single<&mut Text, (With<SimulationStatus>, Without<Status>)>,
) {
    if let Some(message) = control.message.take() {
        editor.message = message;
    }
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
    mut scroll: Query<&mut ScrollPosition, With<Form>>,
    mut send: Single<&mut Node, With<SendForm>>,
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
    let title = match editor.tab {
        "motion" => "MotionCommand",
        "game" => "FilteredGameControllerState",
        _ => "Motion parameters",
    };
    if editor.rendered_tab != editor.tab {
        if let Ok(mut scroll) = scroll.get_mut(*form) {
            scroll.y = 0.0;
        }
        editor.rendered_tab = editor.tab;
    }
    send.display = if editor.tab == "parameters" {
        Display::None
    } else {
        Display::Flex
    };
    if editor.tab == "parameters" {
        let form_entity = *form;
        text(
            &mut commands,
            *form,
            "Session settings. Applying restarts all motion nodes and resets the robot paused. Angles use radians unless named degrees; durations use seconds.",
        );
        commands.spawn_scene(bsn! {
            @FeathersButton ChildOf(form_entity) Children[label("Apply parameters & reset")]
            on(|_: On<Activate>, mut editor: ResMut<Editor>, io: Res<Robotics>,
                inputs: Query<(&ParameterText, &bevy::text::EditableText)>,
                mut control: ResMut<SimulationControl>| {
                if !io.launches_nodes() {
                    editor.message = "Parameter application requires the simulator's robotics nodes (--no-robotics is active).".into();
                    return;
                }
                // Include text edits from this frame even if the polling system has not run yet.
                copy_parameter_text(&mut editor, &inputs);
                match MotionParameters::prepare(editor.draft["parameters"].clone()) {
                    Ok(layer) => {
                        control.parameter_overrides = Some(layer);
                        control.reset = true;
                        editor.message = "Applying parameters and restarting the motion stack...".into();
                    }
                    Err(error) => editor.message = format!("Cannot apply parameters: {error:#}"),
                }
            })
        });
        commands.spawn_scene(bsn! {
            @FeathersButton ChildOf(form_entity) Children[label("Discard edits / reload applied settings")]
            on(|_: On<Activate>, mut editor: ResMut<Editor>, io: Res<Robotics>| {
                match io.parameter_values() {
                    Ok(parameters) => {
                        editor.draft["parameters"] = parameters;
                        editor.rebuild = true;
                        editor.message = "Loaded the current motion settings.".into();
                    }
                    Err(error) => editor.message = format!("Cannot load parameters: {error:#}"),
                }
            })
        });
    }
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

fn copy_parameter_text(
    editor: &mut Editor,
    inputs: &Query<(&ParameterText, &bevy::text::EditableText)>,
) {
    for (path, input) in inputs {
        if let Some(slot) = editor.draft.pointer_mut(&path.0) {
            *slot = Value::String(input.value().to_string());
        }
    }
}

fn sync_parameter_text(
    mut editor: ResMut<Editor>,
    inputs: Query<(&ParameterText, &bevy::text::EditableText)>,
) {
    // Rebuilding (e.g. discarding a draft) must not copy stale widget values back over it.
    if !editor.rebuild {
        copy_parameter_text(&mut editor, &inputs);
    }
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
    let parameter = path.starts_with("/parameters/");
    if parameter && path.matches('/').count() > 2 {
        commands.entity(group).insert(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            width: percent(100),
            flex_shrink: 0.0,
            padding: UiRect::left(px(10)),
            ..default()
        });
    }
    if parameter
        && current.is_object()
        && current.get("secs").is_none()
        && !path.ends_with("/injected_head_joints")
    {
        let open = expanded.contains(path);
        let key = path.to_owned();
        commands.spawn_scene(bsn! {
            @FeathersButton ChildOf(group) Children[label(if open { "Collapse" } else { "Expand" })]
            on(move |_: On<Activate>, mut editor: ResMut<Editor>| {
                if !editor.expanded.remove(&key) { editor.expanded.insert(key.clone()); }
                editor.rebuild = true;
            })
        });
        if !open {
            return;
        }
    }
    if parameter && path.ends_with("/injected_head_joints") {
        let key = path.to_owned();
        let enabled = !current.is_null();
        commands.spawn_scene(bsn! {
            @FeathersButton ChildOf(group) Children[label(if enabled { "Disable override" } else { "Enable override" })]
            on(move |_: On<Activate>, mut editor: ResMut<Editor>| {
                *editor.draft.pointer_mut(&key).unwrap() = if enabled { Value::Null } else { json!({"yaw": 0.0, "pitch": 0.0}) };
                editor.rebuild = true;
            })
        });
        if !enabled {
            return;
        }
    }
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
    if !parameter
        && allow_choices
        && let Some(options) = choices(path)
    {
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
                    &if parameter {
                        key.replace('_', " ")
                    } else {
                        field_label(key)
                    },
                    value,
                    true,
                    expanded,
                );
            }
        }
    } else if let Value::Array(values) = current {
        for (index, value) in values.iter().enumerate() {
            let item_label = if parameter {
                if path.contains("/image_region_parameters/") {
                    ["x (normalized image)", "y (normalized image)"][index].to_owned()
                } else if values.len() == 2 {
                    ["minimum", "maximum"][index].to_owned()
                } else {
                    index.to_string()
                }
            } else if path.ends_with("/segments") {
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
    } else if parameter && let Value::String(initial) = current {
        let path = path.to_owned();
        commands.spawn_scene(bsn! {
            @FeathersTextInputContainer ChildOf(group)
            Node { width: percent(100), min_height: px(28) }
            Children [
                @FeathersTextInput
                template_value(ParameterText(path))
                template_value(bevy::text::EditableText::new(initial.clone()))
            ]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bevy_mujoco::{MjcfObject, MujocoWorldPlugin},
        parameters::BallParameters,
        scene::ball::{self, Ball},
    };

    #[test]
    fn first_ball_target_uses_current_ground_coordinates_and_spawn_order() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, MujocoWorldPlugin));
        app.insert_resource(SimulationMode::Paused);
        app.init_resource::<SpawnedBalls>()
            .add_observer(ball::record_spawn)
            .add_observer(ball::record_removal);
        let robot = app
            .world_mut()
            .spawn((
                MjcfObject::new(
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/k1_robot.xml"),
                    "Trunk",
                )
                .with_free_joint("world_joint")
                .grounded(),
                Transform::from_xyz(2.0, 0.0, -3.0)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            ))
            .id();
        app.update();
        let target = |app: &App| {
            look_at_first_ball(
                app.world().resource::<MujocoWorld>(),
                app.world().resource::<SpawnedBalls>(),
                robot,
            )
        };
        assert!(target(&app).is_err());
        let origin = {
            let data = app.world().resource::<MujocoWorld>().data();
            let feet = ["left_foot_link", "right_foot_link"].map(|foot| {
                let foot = data
                    .body(&format!("object_{}_{foot}", robot.to_bits()))
                    .unwrap()
                    .view(data);
                Vec2::new(foot.xpos[0] as f32, foot.xpos[1] as f32)
            });
            (feet[0] + feet[1]) * 0.5
        };
        let ball_object = || {
            MjcfObject::from_factory(
                || {
                    ball::ball_spec(
                        0.105,
                        &BallParameters {
                            mass: 0.45,
                            joint_damping: 0.002,
                            joint_friction_loss: 0.0,
                            friction: [1.0, 0.005, 0.0001],
                            solref: [0.08, 0.25],
                            solimp: [0.9, 0.95, 0.001, 0.5, 2.0],
                        },
                    )
                },
                "ball",
            )
            .with_free_joint("ball_free_joint")
        };
        // Bevy is Y-up; MuJoCo is Z-up. The robot faces world +Y here.
        let first = app
            .world_mut()
            .spawn((
                Ball,
                ball_object(),
                Transform::from_xyz(origin.x - 2.0, 0.6, -(origin.y + 1.0)),
            ))
            .id();
        let second = app
            .world_mut()
            .spawn((
                Ball,
                ball_object(),
                Transform::from_xyz(origin.x, 0.105, -(origin.y + 4.0)),
            ))
            .id();
        app.update();
        let MotionCommand::Stand {
            head:
                HeadMotion::LookAt {
                    target: point,
                    height_above_ground,
                    image_region_target,
                },
        } = target(&app).unwrap()
        else {
            panic!("expected Stand with LookAt");
        };
        assert!((point.x() - 1.0).abs() < 1e-5);
        assert!((point.y() - 2.0).abs() < 1e-5);
        assert!((height_above_ground - 0.6).abs() < 1e-5);
        assert_eq!(image_region_target, ImageRegion::Center);
        app.world_mut()
            .resource_mut::<MujocoWorld>()
            .set_object_pose(
                first,
                Transform::from_xyz(origin.x - 3.0, 0.105, -(origin.y + 1.0)),
            )
            .unwrap();
        let MotionCommand::Stand {
            head: HeadMotion::LookAt { target: point, .. },
        } = target(&app).unwrap()
        else {
            panic!("expected LookAt");
        };
        assert!(
            (point.y() - 3.0).abs() < 1e-5,
            "must sample current ball position"
        );
        app.world_mut().despawn(first);
        let third = app
            .world_mut()
            .spawn((Ball, ball_object(), Transform::default()))
            .id();
        app.update();
        assert_eq!(
            app.world().resource::<SpawnedBalls>().0,
            vec![second, third]
        );
        let MotionCommand::Stand {
            head: HeadMotion::LookAt { target: point, .. },
        } = target(&app).unwrap()
        else {
            panic!("expected LookAt");
        };
        assert!(
            (point.x() - 4.0).abs() < 1e-5,
            "must select oldest remaining ball"
        );
    }
}
