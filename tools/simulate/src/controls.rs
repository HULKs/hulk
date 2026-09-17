//! Structured editors for the exact published message fields, using native Bevy widgets.
mod choices;

use std::collections::{BTreeMap, HashSet};

use bevy::{
    camera_controller::free_camera::FreeCameraState,
    feathers::{
        FeathersPlugins, controls::*, dark_theme::create_dark_theme, display::label, theme::UiTheme,
    },
    input_focus::{InputFocus, tab_navigation::TabGroup},
    prelude::*,
    ui::InteractionDisabled,
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
    robot_io::RobotBinding,
    robotics::Robotics,
    scene::ball::SpawnedBalls,
    simulation::{ControlledRobot, SimulationControl},
};
use choices::{choices, is_angle, value, variant};

pub const PANEL_WIDTH: f32 = 480.0;

fn look_at_first_ball(
    world: &MujocoWorld,
    balls: &SpawnedBalls,
    robot: Entity,
) -> color_eyre::Result<MotionCommand> {
    let target = first_ball_in_ground(world, balls, robot)?;
    Ok(MotionCommand::Stand {
        head: HeadMotion::LookAt {
            target: point![target.x, target.y],
            height_above_ground: target.z,
            image_region_target: ImageRegion::Center,
        },
    })
}

fn first_ball_in_ground(
    world: &MujocoWorld,
    balls: &SpawnedBalls,
    robot: Entity,
) -> color_eyre::Result<nalgebra::Point3<f32>> {
    let position = crate::scene::ball::first_position(world, balls)?;
    let binding = RobotBinding::new(world.data(), &format!("object_{}_", robot.to_bits()))?;
    Ok(binding.point_in_ground(world.data(), position))
}

fn fill_kick_ball(
    command: &mut MotionCommand,
    world: &MujocoWorld,
    balls: &SpawnedBalls,
    robot: Entity,
) -> color_eyre::Result<()> {
    if let MotionCommand::Kick { ball_position, .. } = command {
        let ball = first_ball_in_ground(world, balls, robot)?;
        *ball_position = point![ball.x, ball.y];
    }
    Ok(())
}

#[derive(Resource)]
struct Editor {
    draft: Value,
    tab: &'static str,
    rebuild: bool,
    message: String,
    expanded: HashSet<String>,
    rendered_tab: String,
    parameter_group: &'static str,
    baselines: BTreeMap<String, crate::motion_parameters::Snapshot>,
    completion: u64,
    pending: Option<(&'static str, Value)>,
    message_error: bool,
    track_ball: bool,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            draft: json!({"motion": value(MotionCommand::Damping), "game": value(FilteredGameControllerState::default()), "parameters": {}}),
            tab: "motion",
            rebuild: true,
            expanded: HashSet::from(["/parameters/head_motion/joint_control".into()]),
            rendered_tab: String::new(),
            parameter_group: "head_motion",
            baselines: BTreeMap::new(),
            completion: 0,
            pending: None,
            message_error: false,
            track_ball: false,
            message: "Edit a command, then press Send. Numbers support dragging and text entry."
                .into(),
        }
    }
}

#[derive(Component)]
struct Form;
#[derive(Component, Default, Clone)]
struct PanelText;
#[derive(Component, Clone)]
enum PanelLabel {
    Run,
    Telemetry,
    Submit,
    Details,
    Draft,
    Status,
    TrackBall,
    Vectors,
    KickBallOrigin,
    ParameterGroup(&'static str),
}
#[derive(Component, Clone)]
struct TabButton(&'static str);
#[derive(Component, Default, Clone)]
struct SubmitButton;
#[derive(Component)]
struct ParameterNavigation;
#[derive(Component, Clone)]
struct ParameterTab(&'static str);
#[derive(Component, Clone)]
struct ParameterText(String);
#[derive(Component)]
struct MotionNumber(String);
#[derive(Component)]
struct MotionReadout(String);

pub struct ControlsPlugin;
impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FeathersPlugins)
            .insert_resource(UiTheme(panel_theme()))
            .init_resource::<Editor>()
            .add_systems(Startup, setup)
            .add_systems(PreUpdate, gate_camera_input)
            .add_systems(
                Update,
                (
                    update_ball_target,
                    sync_parameter_text,
                    synchronize_parameters,
                    rebuild_form,
                    update_motion_readouts,
                    update_status,
                    style_actions,
                    style_panel_text,
                )
                    .chain(),
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
        PanelText,
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
                padding: UiRect::all(px(20)),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            BackgroundColor(rgb(0x202b3b)),
            GlobalZIndex(10),
            TabGroup::default(),
        ))
        .id();
    let header = row(&mut commands, root);
    heading(&mut commands, header, "Motion studio", 24.0);
    let toolbar = row(&mut commands, root);
    commands.spawn_scene(bsn! {
        @FeathersButton { @variant: ButtonVariant::Primary }
        ChildOf(toolbar) Node { height: px(36), min_width: px(105), flex_shrink: 0.0 }
        Children [Text::new("Run") PanelText template_value(PanelLabel::Run)]
        on(|_: On<Activate>, mut mode: ResMut<SimulationMode>| {
            *mode = match *mode { SimulationMode::Paused => SimulationMode::Running, SimulationMode::Running => SimulationMode::Paused };
        })
    });
    commands.spawn_scene(bsn! {
        @FeathersButton ChildOf(toolbar) Node { height: px(36), flex_grow: 1.0 }
        Children[label("Reset robot & stack")]
        on(|_: On<Activate>, mut control: ResMut<SimulationControl>| { control.reset = true; })
    });
    panel_label(&mut commands, root, PanelLabel::Telemetry);
    let tabs = row(&mut commands, root);
    for (name, title) in [
        ("motion", "Commands"),
        ("game", "Game"),
        ("parameters", "Parameters"),
    ] {
        commands.spawn_scene(bsn! {
            @FeathersButton ChildOf(tabs) template_value(TabButton(name))
            Node { height: px(36), flex_grow: 1.0, flex_basis: px(0) }
            Children[label(title)]
            on(move |_: On<Activate>, mut editor: ResMut<Editor>| {
                editor.tab = name; editor.rebuild = true; editor.message.clear();
            })
        });
    }
    let groups = row(&mut commands, root);
    commands.entity(groups).insert(ParameterNavigation);
    for &(key, title, _, _) in &crate::motion_parameters::GROUPS {
        commands.spawn_scene(bsn! {
            @FeathersButton ChildOf(groups) template_value(ParameterTab(key))
            Node { height: px(30), flex_grow: 1.0 }
            Children[Text::new(title) PanelText template_value(PanelLabel::ParameterGroup(key)) TextFont { font_size: bevy::text::FontSize::Px(14.0) }]
            on(move |_: On<Activate>, mut editor: ResMut<Editor>| { editor.parameter_group = key; editor.rebuild = true; })
        });
    }
    panel_label(&mut commands, root, PanelLabel::Details);
    panel_label(&mut commands, root, PanelLabel::Vectors);
    commands
        .spawn((
            ChildOf(root),
            Form,
            Node {
                overflow: Overflow::scroll_y(),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                padding: UiRect::right(px(8)),
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
                                32.0
                            } else {
                                1.0
                            })
                    .clamp(0.0, max.max(0.0));
                    event.propagate(false);
                }
            },
        );
    let footer = column(&mut commands, root);
    commands
        .entity(footer)
        .insert(Node {
            width: percent(100),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::top(px(12)),
            border: UiRect::top(px(1)),
            ..default()
        })
        .insert(BorderColor::all(rgb(0x43556d)));
    panel_label(&mut commands, footer, PanelLabel::Draft);
    panel_label(&mut commands, footer, PanelLabel::Status);
    let actions = row(&mut commands, footer);
    commands.spawn_scene(bsn! {
        @FeathersButton ChildOf(actions) Node { height: px(36), flex_grow: 1.0 }
        Children[label("Discard edits")]
        on(|_: On<Activate>, mut editor: ResMut<Editor>, io: Res<Robotics>| {
            if editor.tab == "parameters" {
                let group = editor.parameter_group;
                if let Some(snapshot) = io.parameters.state().snapshots.get(group) {
                    editor.draft["parameters"][group] = snapshot.value.clone();
                    editor.baselines.insert(group.to_string(), snapshot.clone());
                }
            } else if editor.tab == "motion" { editor.draft["motion"] = value(&io.input_motion); }
            else { editor.draft["game"] = value(&io.input_game); }
            editor.rebuild = true;
            editor.message = "Loaded current values".into();
            editor.message_error = false;
        })
    });
    commands.spawn_scene(bsn! {
        @FeathersButton { @variant: ButtonVariant::Primary }
        ChildOf(actions) SubmitButton Node { height: px(36), flex_grow: 1.0 }
        Children[Text::new("Send command") PanelText template_value(PanelLabel::Submit)]
        on(|_: On<Activate>, mut editor: ResMut<Editor>, mut io: ResMut<Robotics>, inputs: Query<(&ParameterText, &bevy::text::EditableText)>, world: Res<MujocoWorld>, balls: Res<SpawnedBalls>, robot: Single<Entity, With<ControlledRobot>>| {
            if editor.tab == "motion" && matches!(serde_json::from_value::<MotionCommand>(editor.draft["motion"].clone()), Ok(MotionCommand::Walk { .. })) {
                editor.message_error = true;
                editor.message = "Path-based walking is not implemented in motion yet. Use Walk with velocity.".into();
                return;
            }
            copy_parameter_text(&mut editor, &inputs);
            let result = if editor.tab == "parameters" {
                let group = editor.parameter_group;
                if let Some(snapshot) = editor.baselines.get(group) {
                    let draft = editor.draft["parameters"][group].clone();
                    io.parameters.apply(group, draft.clone(), snapshot).map(|()| {
                        editor.pending = Some((group, draft));
                        "Applying live...".to_owned()
                    })
                } else { Err(color_eyre::eyre::eyre!("Waiting for the node's parameter service")) }
            } else {
                let decoded = if editor.tab == "motion" {
                    serde_json::from_value::<MotionCommand>(editor.draft["motion"].clone()).map_err(color_eyre::Report::from).and_then(|mut command| {
                        fill_kick_ball(&mut command, &world, &balls, *robot)?;
                        editor.track_ball = false;
                        editor.draft["motion"] = value(&command);
                        io.input_motion = command;
                        Ok(())
                    })
                } else {
                    serde_json::from_value::<FilteredGameControllerState>(editor.draft["game"].clone()).map(|game| io.input_game = game).map_err(color_eyre::Report::from)
                };
                decoded.and_then(|()| io.publish_inputs()).map(|()| "Published".to_owned())
            };
            editor.message_error = result.is_err();
            editor.message = result.unwrap_or_else(|e| format!("{e:#}"));
        })
    });
}

fn row(commands: &mut Commands, parent: Entity) -> Entity {
    commands
        .spawn((
            ChildOf(parent),
            Node {
                width: percent(100),
                column_gap: px(8),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id()
}

fn heading(commands: &mut Commands, parent: Entity, content: &str, size: f32) {
    commands.spawn((
        ChildOf(parent),
        Text::new(content),
        PanelText,
        TextFont::from_font_size(size),
        TextColor(rgb(0xecf2fa)),
    ));
}

fn panel_label(commands: &mut Commands, parent: Entity, label: PanelLabel) {
    commands.spawn((
        ChildOf(parent),
        Text::new(""),
        PanelText,
        label,
        TextFont::from_font_size(13.0),
        TextColor(rgb(0xb4c5da)),
    ));
}

fn rgb(hex: u32) -> Color {
    Color::srgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

fn panel_theme() -> bevy::feathers::theme::ThemeProps {
    use bevy::feathers::tokens::semantic::*;
    let mut theme = create_dark_theme();
    for (token, color) in [
        (FILL_ACCENT_DEFAULT, 0x346ea6),
        (FILL_ACCENT_HOVER, 0x4383bb),
        (FILL_ACCENT_PRESSED, 0x295d91),
        (FILL_SOLID_DEFAULT, 0x35465c),
        (FILL_SOLID_HOVER, 0x445b75),
        (FILL_SOLID_PRESSED, 0x293a4f),
        (FILL_FIELD_DEFAULT, 0x17212e),
        (FILL_FIELD_HOVER, 0x26354a),
        (TEXT_DEFAULT, 0xecf2fa),
        (TEXT_DIM, 0xb4c5da),
        (FOCUS_RING, 0x77b3f3),
    ] {
        theme.semantic_base.insert(token, rgb(color));
    }
    theme
}

fn style_panel_text(mut text: Query<&mut TextFont, Added<PanelText>>, assets: Res<AssetServer>) {
    for mut font in &mut text {
        font.font = assets
            .load(bevy::feathers::constants::fonts::REGULAR)
            .into();
    }
}

fn style_actions(
    editor: Res<Editor>,
    io: Res<Robotics>,
    mut tabs: Query<(&TabButton, &mut ButtonVariant), Without<ParameterTab>>,
    mut groups: Query<(&ParameterTab, &mut ButtonVariant), Without<TabButton>>,
    mut navigation: Single<&mut Node, With<ParameterNavigation>>,
    submit: Single<Entity, With<SubmitButton>>,
    mut commands: Commands,
) {
    let state = io.parameters.state();
    let group = editor.parameter_group;
    let baseline = editor.baselines.get(group);
    let dirty =
        baseline.is_some_and(|snapshot| editor.draft["parameters"][group] != snapshot.value);
    let disconnected = state.errors.contains_key(group) || baseline.is_none();
    let busy = state.busy || editor.pending.is_some();
    let disabled = editor.tab == "parameters" && (busy || disconnected || !dirty);
    if disabled {
        commands.entity(*submit).insert(InteractionDisabled);
    } else {
        commands.entity(*submit).remove::<InteractionDisabled>();
    }
    for (tab, mut variant) in &mut tabs {
        *variant = if tab.0 == editor.tab {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Plain
        };
    }
    navigation.display = if editor.tab == "parameters" {
        Display::Flex
    } else {
        Display::None
    };
    for (tab, mut variant) in &mut groups {
        *variant = if tab.0 == editor.parameter_group {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Normal
        };
    }
}

fn update_status(
    mut editor: ResMut<Editor>,
    mut control: ResMut<SimulationControl>,
    io: Res<Robotics>,
    world: Res<MujocoWorld>,
    mode: Res<SimulationMode>,
    balls: Res<SpawnedBalls>,
    mut labels: Query<(&PanelLabel, &mut Text, &mut TextColor)>,
) {
    if let Some(message) = control.message.take() {
        editor.message = message;
        editor.message_error = false;
        editor.pending = None;
    }
    let state = io.parameters.state();
    let group = editor.parameter_group;
    let baseline = editor.baselines.get(group);
    let dirty = if editor.tab == "parameters" {
        baseline.is_some_and(|snapshot| editor.draft["parameters"][group] != snapshot.value)
    } else if editor.tab == "motion" {
        editor.draft["motion"] != value(&io.input_motion)
    } else {
        editor.draft["game"] != value(&io.input_game)
    };
    let disconnected = state.errors.contains_key(group) || baseline.is_none();
    let busy = state.busy || editor.pending.is_some();
    for (kind, mut label, mut color) in &mut labels {
        label.0 = match kind {
            PanelLabel::Vectors => match &io.input_motion {
                MotionCommand::WalkWithVelocity { .. } => {
                    "Sent vectors: blue = velocity, green = yaw rate\n1 m = 1 m/s or 1 rad/s".into()
                }
                MotionCommand::Kick { .. } => {
                    "Sent vector: amber = kick direction from ball (1 m)".into()
                }
                _ => "Command vectors appear after sending a walk or kick.".into(),
            },
            PanelLabel::KickBallOrigin => if balls.0.is_empty() {
                "No ball — spawn one before sending a kick"
            } else {
                "Ground truth · first ball · updated live"
            }
            .into(),
            PanelLabel::TrackBall => if editor.track_ball {
                "Stop tracking ball"
            } else {
                "Look at first ball"
            }
            .into(),
            PanelLabel::ParameterGroup(key) => {
                let title = crate::motion_parameters::GROUPS
                    .iter()
                    .find(|group| group.0 == *key)
                    .unwrap()
                    .1;
                let changed = editor
                    .baselines
                    .get(*key)
                    .is_some_and(|snapshot| editor.draft["parameters"][*key] != snapshot.value);
                format!("{title}{}", if changed { " *" } else { "" })
            }
            PanelLabel::Run => if *mode == SimulationMode::Paused {
                "Run"
            } else {
                "Pause"
            }
            .into(),
            PanelLabel::Telemetry => format!(
                "{}     {:.3} s     Joints: {}",
                if *mode == SimulationMode::Paused {
                    "Paused"
                } else {
                    "Running"
                },
                world.data().time(),
                if io.latest_command().is_some() {
                    "connected"
                } else {
                    "waiting"
                }
            ),
            PanelLabel::Submit => if editor.tab == "parameters" {
                if busy { "Applying..." } else { "Apply live" }
            } else if editor.tab == "motion" {
                "Send command"
            } else {
                "Send game state"
            }
            .into(),
            PanelLabel::Details => match editor.tab {
                "motion" => "Controls body and head. Use Walk with velocity for walking.".into(),
                "game" => "Set match state and field side for head-motion tests.".into(),
                _ => "Tune the running nodes. Applying keeps the robot and simulation running."
                    .into(),
            },
            PanelLabel::Draft => {
                if editor.tab == "motion" && editor.track_ball {
                    "Tracking first ball · target updates live".into()
                } else if editor.tab == "parameters" && disconnected {
                    "Waiting for parameter service...".into()
                } else if editor.tab == "parameters"
                    && baseline
                        .zip(state.snapshots.get(group))
                        .is_some_and(|(old, new)| old.revision != new.revision)
                {
                    "Node changed externally. Discard edits to reload before applying.".into()
                } else if dirty {
                    "Unapplied changes".into()
                } else {
                    "Up to date".into()
                }
            }
            PanelLabel::Status => {
                let stack = io.status();
                let message = if stack.contains("failed")
                    || stack.contains("exited")
                    || stack.contains("fault")
                {
                    stack
                } else if editor.tab == "parameters" && disconnected {
                    state.errors.get(group).cloned().unwrap_or_default()
                } else {
                    editor.message.clone()
                };
                color.0 = if editor.message_error {
                    rgb(0xe6b66b)
                } else {
                    rgb(0xb4c5da)
                };
                message.chars().take(350).collect()
            }
        };
    }
}

fn synchronize_parameters(
    mut editor: ResMut<Editor>,
    io: Res<Robotics>,
    focus: Res<InputFocus>,
    inputs: Query<(), With<bevy::text::EditableText>>,
) {
    let editing = focus.get().is_some_and(|entity| inputs.contains(entity));
    let state = io.parameters.state();
    if state.completion != editor.completion {
        editor.completion = state.completion;
        if let Some(result) = &state.result {
            editor.message_error = result.is_err();
            editor.message = result.clone().unwrap_or_else(|e| e);
            if let Some((group, submitted)) = editor.pending.take()
                && result.is_ok()
                && let Some(snapshot) = state.snapshots.get(group)
            {
                if editor.draft["parameters"][group] == submitted {
                    editor.draft["parameters"][group] = snapshot.value.clone();
                    editor.rebuild = true;
                }
                editor.baselines.insert(group.into(), snapshot.clone());
            }
        }
    }
    for (group, snapshot) in &state.snapshots {
        let baseline = editor.baselines.get(group);
        let clean = baseline.is_none_or(|old| editor.draft["parameters"][group] == old.value);
        if clean && !editing && baseline != Some(snapshot) && editor.pending.is_none() {
            let changed = editor.draft["parameters"][group] != snapshot.value;
            editor.draft["parameters"][group] = snapshot.value.clone();
            editor.baselines.insert(group.clone(), snapshot.clone());
            if changed && editor.tab == "parameters" {
                editor.rebuild = true;
            }
        }
    }
}

fn update_ball_target(
    mut commands: Commands,
    world: Res<MujocoWorld>,
    balls: Res<SpawnedBalls>,
    robot: Single<Entity, With<ControlledRobot>>,
    mut io: ResMut<Robotics>,
    mut editor: ResMut<Editor>,
    numbers: Query<(Entity, &MotionNumber, &NumberInputValue)>,
) {
    let draft_kick = editor.draft["motion"].get("Kick").is_some();
    let active_kick = matches!(io.input_motion, MotionCommand::Kick { .. });
    if draft_kick || active_kick {
        match first_ball_in_ground(&world, &balls, *robot) {
            Ok(ball) => {
                let ball = point![ball.x, ball.y];
                if draft_kick {
                    editor.draft["motion"]["Kick"]["ball_position"] = value(ball);
                }
                if let MotionCommand::Kick { ball_position, .. } = &mut io.input_motion
                    && *ball_position != ball
                {
                    *ball_position = ball;
                    if let Err(error) = io.publish_inputs() {
                        editor.message_error = true;
                        editor.message = format!("Could not update kick ball position: {error}");
                    }
                }
            }
            Err(error) if active_kick => {
                io.input_motion = MotionCommand::Damping;
                let result = io.publish_inputs();
                editor.message_error = true;
                editor.message = match result {
                    Ok(()) => format!("Kick stopped: {error}"),
                    Err(publish_error) => {
                        format!("Kick stopped: {error}; publishing failed: {publish_error}")
                    }
                };
            }
            Err(_) => {}
        }
    }
    if !editor.track_ball {
        return;
    }
    let result = look_at_first_ball(&world, &balls, *robot).and_then(|command| {
        let next = value(&command);
        if next != value(&io.input_motion) {
            io.input_motion = command;
            io.publish_inputs()?;
        }
        editor.draft["motion"] = next;
        // Update the displayed coordinates in place so tracking does not recreate
        // the form or interrupt clicks. This also runs while dragging pauses physics.
        for (entity, path, input) in &numbers {
            if let Some(number) = editor.draft.pointer(&path.0).and_then(Value::as_f64) {
                let next = NumberInputValue::F64(number);
                if *input != next {
                    commands.entity(entity).insert(next);
                }
            }
        }
        Ok(())
    });
    if let Err(error) = result {
        editor.track_ball = false;
        editor.message_error = true;
        editor.message = format!("Ball tracking stopped: {error}");
    }
}

fn update_motion_readouts(editor: Res<Editor>, mut readouts: Query<(&MotionReadout, &mut Text)>) {
    for (path, mut text) in &mut readouts {
        if let Some(number) = editor.draft.pointer(&path.0).and_then(Value::as_f64) {
            text.set_if_neq(Text::new(format!("{number:.3}")));
        }
    }
}

fn rebuild_form(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    form: Single<Entity, With<Form>>,
    children: Query<&Children>,
    mut scroll: Query<&mut ScrollPosition, With<Form>>,
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
    let form = *form;
    let rendered = format!("{}/{}", editor.tab, editor.parameter_group);
    if editor.rendered_tab != rendered {
        if let Ok(mut scroll) = scroll.get_mut(form) {
            scroll.y = 0.0;
        }
        editor.rendered_tab = rendered;
    }
    if editor.tab == "parameters" {
        let group = editor.parameter_group;
        if editor.baselines.contains_key(group) {
            build_field(
                &mut commands,
                form,
                &format!("/parameters/{group}"),
                "",
                &editor.draft["parameters"][group],
                false,
                &editor.expanded,
            );
        } else {
            text(&mut commands, form, "Connecting to the running node...");
        }
    } else {
        if editor.tab == "motion" {
            let shortcuts = row(&mut commands, form);
            commands.spawn_scene(bsn! {
                @FeathersButton ChildOf(shortcuts) Node { height: px(32), flex_grow: 1.0 }
                Children[Text::new("Look at first ball") PanelText template_value(PanelLabel::TrackBall)]
                on(|_: On<Activate>, world: Res<MujocoWorld>, balls: Res<SpawnedBalls>, robot: Single<Entity, With<ControlledRobot>>, mut io: ResMut<Robotics>, mut editor: ResMut<Editor>| {
                    if editor.track_ball {
                        editor.track_ball = false;
                        editor.message_error = false;
                        editor.message = "Ball tracking stopped; holding the last target".into();
                        return;
                    }
                    let result = look_at_first_ball(&world, &balls, *robot).and_then(|command| {
                        editor.draft["motion"] = value(&command); editor.rebuild = true; io.input_motion = command; io.publish_inputs()
                    });
                    editor.track_ball = result.is_ok();
                    editor.message_error = result.is_err();
                    editor.message = result.map_or_else(|e| e.to_string(), |()| "Tracking the first ball. Move it to update the target.".into());
                })
            });
            commands.spawn_scene(bsn! {
                @FeathersButton ChildOf(shortcuts) Node { height: px(32), flex_grow: 1.0 }
                Children[label("Damp robot")]
                on(|_: On<Activate>, mut io: ResMut<Robotics>, mut editor: ResMut<Editor>| {
                    editor.track_ball = false;
                    io.input_motion = MotionCommand::Damping; editor.draft["motion"] = value(&io.input_motion); editor.rebuild = true;
                    let result = io.publish_inputs(); editor.message_error = result.is_err();
                    editor.message = result.map_or_else(|e| e.to_string(), |()| "Robot damping sent".into());
                })
            });
        }
        let path = format!("/{}", editor.tab);
        build_field(
            &mut commands,
            form,
            &path,
            if editor.tab == "motion" {
                "Behavior request"
            } else {
                "Match settings"
            },
            &editor.draft[editor.tab],
            true,
            &editor.expanded,
        );
    }
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
    let name = readable(variant(&option));
    let path = path.to_owned();
    commands.spawn_scene(bsn! {
        @FeathersButton { @variant: {if selected { ButtonVariant::Primary } else { ButtonVariant::Normal }} }
        ChildOf(parent) Node { min_height: px(30), flex_shrink: 0.0 } Children[label(&name)]
        on(move |_: On<Activate>, mut editor: ResMut<Editor>| {
            if path.starts_with("/motion/") { editor.track_ball = false; }
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
    if path == "/motion/Kick/ball_position" {
        panel_label(commands, parent, PanelLabel::KickBallOrigin);
    }
    let parameter = path.starts_with("/parameters/");
    let title = readable(title);
    if let Some((initial, kind)) = numeric_value(path, current) {
        let group = row(commands, parent);
        let caption = column(commands, group);
        commands.entity(caption).insert(Node {
            width: percent(59),
            flex_shrink: 1.0,
            ..default()
        });
        text(
            commands,
            caption,
            if matches!(kind, Numeric::Duration) {
                format!("{title} (s)")
            } else {
                title
            },
        );
        let input = column(commands, group);
        commands.entity(input).insert(Node {
            width: percent(41),
            min_width: px(90),
            ..default()
        });
        number(commands, input, path, initial, kind);
        return;
    }
    let group = column(commands, parent);
    let pairs: Option<Vec<(String, &Value)>> = match current {
        Value::Object(fields)
            if !fields.is_empty()
                && fields.len() <= 4
                && fields.values().all(Value::is_number)
                && fields
                    .keys()
                    .all(|key| matches!(key.as_str(), "yaw" | "pitch" | "x" | "y" | "z" | "w"))
                && !path.ends_with("/injected_head_joints") =>
        {
            Some(
                fields
                    .iter()
                    .map(|(key, value)| (key.clone(), value))
                    .collect(),
            )
        }
        Value::Array(values)
            if !values.is_empty() && values.len() <= 4 && values.iter().all(Value::is_number) =>
        {
            Some(
                values
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (i.to_string(), v))
                    .collect(),
            )
        }
        _ => None,
    };
    if let Some(pairs) = pairs {
        text(commands, group, &title);
        let inputs = row(commands, group);
        let count = pairs.len();
        for (key, value) in pairs {
            let cell = column(commands, inputs);
            commands.entity(cell).insert(Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            });
            let label = if current.is_array() {
                let index: usize = key.parse().unwrap();
                if parameter && count == 2 && !path.contains("/image_region_parameters/") {
                    ["Min", "Max"][index]
                } else {
                    ["x", "y", "z", "w"][index]
                }
            } else {
                &key
            };
            text(commands, cell, readable(label));
            number(
                commands,
                cell,
                &format!("{path}/{key}"),
                value.as_f64().unwrap(),
                Numeric::Float,
            );
        }
        return;
    }
    let collapsible = (parameter
        && path.matches('/').count() > 2
        && current.is_object()
        && !path.ends_with("/injected_head_joints"))
        || path.ends_with("/penalties")
        || path.ends_with("_penalties_last_cycle");
    if collapsible {
        let open = expanded.contains(path);
        let key = path.to_owned();
        let caption = format!("{}  {title}", if open { "−" } else { "+" });
        commands.spawn_scene(bsn! {
            @FeathersButton { @variant: ButtonVariant::Plain }
            ChildOf(group) Node { height: px(34), justify_content: JustifyContent::FlexStart, width: percent(100) }
            Children[label(&caption)]
            on(move |_: On<Activate>, mut editor: ResMut<Editor>| {
                if !editor.expanded.remove(&key) { editor.expanded.insert(key.clone()); }
                editor.rebuild = true;
            })
        });
        if !open {
            return;
        }
    } else if !title.is_empty() {
        heading(commands, group, &title, 15.0);
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
            Node { width: percent(100), min_height: px(30) }
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

fn readable(name: &str) -> String {
    let mut result = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch == '_' {
            result.push(' ');
        } else {
            if index > 0 && ch.is_uppercase() && !result.ends_with(' ') {
                result.push(' ');
            }
            result.push(ch);
        }
    }
    if let Some(first) = result.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    result
}

fn numeric_value(path: &str, value: &Value) -> Option<(f64, Numeric)> {
    if let Some(number) = value.as_f64() {
        let unsigned = if path.starts_with("/parameters/") {
            path.ends_with("/inference_threads")
        } else {
            value.as_number()?.is_u64()
        };
        return Some((
            number,
            if unsigned {
                Numeric::Unsigned
            } else {
                Numeric::Float
            },
        ));
    }
    if value.get("secs").is_some() && value.get("nanos").is_some() {
        let duration: std::time::Duration = serde_json::from_value(value.clone()).ok()?;
        return Some((duration.as_secs_f64(), Numeric::Duration));
    }
    if is_angle(path, value) {
        let angle = serde_json::from_value::<Orientation2<Ground>>(value.clone())
            .ok()?
            .angle();
        return Some((f64::from(angle), Numeric::Angle));
    }
    None
}

#[derive(Clone, Copy)]
enum Numeric {
    Float,
    Unsigned,
    Angle,
    Duration,
}
fn number(commands: &mut Commands, parent: Entity, path: &str, initial: f64, kind: Numeric) {
    if path.starts_with("/motion/Kick/ball_position/") {
        // Ground truth is display-only. Disabled Feathers number inputs enqueue
        // child updates on removal, which panic when rebuilding their parent form.
        let container = commands
            .spawn((
                ChildOf(parent),
                Node {
                    width: percent(100),
                    min_height: px(30),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(rgb(0x17212e)),
            ))
            .id();
        commands.spawn((
            ChildOf(container),
            Text::new(format!("{initial:.3}")),
            PanelText,
            MotionReadout(path.to_owned()),
            TextFont::from_font_size(14.0),
            TextColor(rgb(0xb4c5da)),
        ));
        return;
    }
    let motion_path = path.starts_with("/motion/").then(|| path.to_owned());
    let path = path.to_owned();
    let units = match kind {
        Numeric::Angle => "rad",
        Numeric::Duration => "s",
        _ => "",
    };
    if !units.is_empty() && !matches!(kind, Numeric::Duration) {
        text(commands, parent, units);
    }
    let input = commands.spawn_scene(bsn! {
        @FeathersNumberInput
        ChildOf(parent)
        template_value(NumberInputValue::F64(initial))
        NumberInputPrecision(4)
        NumberInputStep(0.01)
        Node { width: percent(100), min_height: px(30) }
        on(move |event: On<ValueChange<f64>>, mut commands: Commands, mut editor: ResMut<Editor>| {
            let n = event.value;
            if !n.is_finite() || n.abs() > f32::MAX as f64 || matches!(kind, Numeric::Unsigned | Numeric::Duration) && n < 0.0 { return; }
            if path.starts_with("/motion/") { editor.track_ball = false; }
            let next = match kind {
                Numeric::Float => json!(n),
                Numeric::Unsigned => json!(n.round() as u64),
                Numeric::Angle => value(Orientation2::<Ground>::new(n as f32)),
                Numeric::Duration => match std::time::Duration::try_from_secs_f64(n) { Ok(d) => value(d), Err(_) => return },
            };
            *editor.draft.pointer_mut(&path).unwrap() = next;
            commands.entity(event.event_target()).insert(NumberInputValue::F64(n));
        })
    }).id();
    if let Some(path) = motion_path {
        commands.entity(input).insert(MotionNumber(path));
    }
}

fn field_label(name: &str) -> String {
    match name {
        "velocity" => "velocity (Ground, m/s)",
        "ball_velocity" => "ball velocity (Ground, m/s)",
        "target_speed" => "target speed (m/s)",
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
    fn changing_kick_options_rebuilds_the_form_without_stale_widget_commands() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::scene::ScenePlugin,
            NumberInputPlugin,
        ))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_resource(UiTheme(panel_theme()))
        .init_resource::<Editor>()
        .add_systems(Update, (rebuild_form, update_motion_readouts).chain());
        app.world_mut().spawn((Form, Node::default()));
        app.world_mut().resource_mut::<Editor>().draft["motion"] = choices("/motion")
            .unwrap()
            .into_iter()
            .find(|option| option.get("Kick").is_some())
            .unwrap();
        app.update();

        for strong in [true, false, true] {
            let group = app
                .world_mut()
                .query::<(&Text, &ChildOf)>()
                .iter(app.world())
                .find_map(|(text, parent)| (text.0 == "Strong").then_some(parent.parent()))
                .unwrap();
            let button = app
                .world_mut()
                .query_filtered::<(Entity, &ChildOf), With<FeathersButton>>()
                .iter(app.world())
                .find_map(|(entity, parent)| (parent.parent() == group).then_some(entity))
                .unwrap();
            app.world_mut().trigger(Activate { entity: button });
            app.update();
            assert_eq!(
                app.world().resource::<Editor>().draft["motion"]["Kick"]["strong"],
                strong
            );
            assert!(app.world().get_entity(button).is_err());
        }

        // Moving the ball must still update both coordinates without rebuilding.
        let readouts: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<MotionReadout>>()
            .iter(app.world())
            .collect();
        assert_eq!(readouts.len(), 2);
        app.world_mut().resource_mut::<Editor>().draft["motion"]["Kick"]["ball_position"] =
            json!([1.23456, -2.34567]);
        app.update();
        for entity in readouts {
            let path = &app.world().get::<MotionReadout>(entity).unwrap().0;
            let expected = if path.ends_with("/0") {
                "1.235"
            } else {
                "-2.346"
            };
            assert_eq!(app.world().get::<Text>(entity).unwrap().0, expected);
        }

        // Leaving the kick form also removes all of its live readouts safely.
        let mut editor = app.world_mut().resource_mut::<Editor>();
        editor.draft["motion"] = value(MotionCommand::Damping);
        editor.rebuild = true;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&MotionReadout>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn head_and_kick_track_ground_truth_ball_positions_while_paused() {
        use ros_z::{
            context::ContextBuilder,
            time::{Clock, Time},
        };
        use std::{path::PathBuf, time::Duration};

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("tcp/127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);
        let (server, io, motion) = runtime.block_on(async {
            let server = ContextBuilder::default()
                .with_mode("router")
                .disable_multicast_scouting()
                .with_connect_endpoints(std::iter::empty::<&str>())
                .with_listen_endpoints([endpoint.as_str()])
                .build()
                .await
                .unwrap();
            let observer = server
                .create_node("ball_tracking_test")
                .build()
                .await
                .unwrap();
            let motion = observer
                .subscriber::<MotionCommand>("/ball_tracking/behavior/motion_command")
                .build()
                .await
                .unwrap();
            let io = Robotics::new(
                runtime.handle().clone(),
                crate::robotics::StackConfiguration {
                    router: endpoint,
                    namespace: "/ball_tracking".into(),
                    parameter_layers: vec![
                        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("parameters"),
                        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../etc/parameters/base"),
                    ],
                    launch_nodes: false,
                },
                Clock::logical(Time::zero()),
            )
            .await
            .unwrap();
            (server, io, motion)
        });
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, MujocoWorldPlugin));
        app.insert_resource(io)
            .init_resource::<Editor>()
            .add_systems(Update, update_ball_target);
        app.insert_resource(SimulationMode::Paused);
        app.init_resource::<SpawnedBalls>()
            .add_observer(ball::record_spawn)
            .add_observer(ball::record_removal);
        let robot = app
            .world_mut()
            .spawn((
                ControlledRobot,
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
        app.world_mut().resource_mut::<Editor>().track_ball = true;
        let height_input = app
            .world_mut()
            .spawn((
                MotionNumber("/motion/Stand/head/LookAt/height_above_ground".into()),
                NumberInputValue::F64(0.0),
            ))
            .id();
        app.update();
        let receive = || {
            runtime.block_on(async {
                tokio::time::timeout(Duration::from_secs(2), motion.recv())
                    .await
                    .unwrap()
                    .unwrap()
            })
        };
        assert_eq!(
            value(receive()),
            value(&app.world().resource::<Robotics>().input_motion)
        );
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
        app.update();
        assert_eq!(value(receive()), value(target(&app).unwrap()));
        assert_eq!(
            app.world().resource::<Editor>().draft["motion"],
            value(target(&app).unwrap())
        );
        let NumberInputValue::F64(height) =
            app.world().get::<NumberInputValue>(height_input).unwrap()
        else {
            panic!("expected a floating-point height input");
        };
        assert!(
            (height - 0.105).abs() < 1e-5,
            "displayed height must track the ball too"
        );
        assert_eq!(
            *app.world().resource::<SimulationMode>(),
            SimulationMode::Paused
        );
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
        assert_eq!(value(receive()), value(target(&app).unwrap()));
        app.world_mut().resource_mut::<Editor>().track_ball = false;
        let held = value(&app.world().resource::<Robotics>().input_motion);
        app.world_mut()
            .resource_mut::<MujocoWorld>()
            .set_object_pose(second, Transform::from_xyz(5.0, 0.105, 5.0))
            .unwrap();
        app.update();
        assert_eq!(
            value(&app.world().resource::<Robotics>().input_motion),
            held
        );
        app.world_mut().despawn(second);
        app.world_mut().despawn(third);
        app.world_mut().resource_mut::<Editor>().track_ball = true;
        app.update();
        assert!(!app.world().resource::<Editor>().track_ball);
        assert!(app.world().resource::<Editor>().message.contains("No ball"));
        assert_eq!(
            value(&app.world().resource::<Robotics>().input_motion),
            held
        );

        let mut kick = MotionCommand::Kick {
            head: HeadMotion::ZeroAngles,
            ball_position: point![99.0, 99.0],
            kick_direction: Orientation2::new(0.3),
            target_position: point![2.0, 0.0],
            robot_theta_to_field: Orientation2::identity(),
            target_speed: 2.7,
            ball_velocity: linear_algebra::vector![0.15, -0.2],
            soft: true,
            quick: true,
            strong: true,
        };
        assert!(
            fill_kick_ball(
                &mut kick,
                app.world().resource::<MujocoWorld>(),
                app.world().resource::<SpawnedBalls>(),
                robot
            )
            .is_err()
        );
        let ball = app
            .world_mut()
            .spawn((
                Ball,
                ball_object(),
                Transform::from_xyz(origin.x - 1.0, 0.105, -(origin.y + 2.0)),
            ))
            .id();
        app.update();
        app.world_mut().resource_mut::<Editor>().draft["motion"] = value(&kick);
        app.world_mut().resource_mut::<Robotics>().input_motion = kick.clone();
        app.update();
        let received = receive();
        let MotionCommand::Kick {
            ball_position,
            kick_direction,
            target_speed,
            ball_velocity,
            soft,
            quick,
            strong,
            ..
        } = &received
        else {
            panic!("expected a kick");
        };
        assert!((ball_position.x() - 2.0).abs() < 1e-5 && (ball_position.y() - 1.0).abs() < 1e-5);
        assert!((kick_direction.angle() - 0.3).abs() < 1e-6);
        assert_eq!(*target_speed, 2.7);
        assert_eq!(*ball_velocity, linear_algebra::vector![0.15, -0.2]);
        assert!(*soft && *quick && *strong);
        assert_eq!(
            app.world().resource::<Editor>().draft["motion"],
            value(&received)
        );
        app.world_mut()
            .resource_mut::<MujocoWorld>()
            .set_object_pose(
                ball,
                Transform::from_xyz(origin.x - 2.0, 0.105, -(origin.y + 3.0)),
            )
            .unwrap();
        app.update();
        let received = receive();
        let MotionCommand::Kick { ball_position, .. } = received else {
            panic!("expected a kick");
        };
        assert!((ball_position.x() - 3.0).abs() < 1e-5 && (ball_position.y() - 2.0).abs() < 1e-5);
        app.world_mut().despawn(ball);
        app.update();
        assert_eq!(receive(), MotionCommand::Damping);
        assert!(
            app.world()
                .resource::<Editor>()
                .message
                .contains("Kick stopped")
        );
        drop(app);
        drop(server);
    }
}
