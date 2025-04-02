# Overview

The behavior of the robot is mainly controlled in the `node` node.
Here it is decided which actions we would like to take.
This is organized in two steps:

## Action Collection

In this step, the world state and other inputs are used to create a priority sorted list of actions, which we would like to take.

!!! example

    ```rust
    let mut actions = vec![
                Action::Unstiff,
                Action::SitDown,
                Action::Penalize,
                Action::Initial,
                Action::FallSafely,
                Action::StandUp,
            ];

    ```

Depending on the current situation, different actions are added to the list of available actions.

!!! example

    If the current role is keeper and a penalty shootout is happening, the actions `Jump` and `PrepareJump` are added.

    ``` rust
        match world_state.robot.role {
            Role::Keeper => match world_state.filtered_game_controller_state {
                Some(FilteredGameControllerState {
                    game_phase: GamePhase::PenaltyShootout { .. },
                    ..
                }) => {
                    actions.push(Action::Jump);
                    actions.push(Action::PrepareJump);
                }
                _ => actions.push(Action::DefendGoal),
            },
            ...
    ```

## Action Selection

Now, the list of actions is iterated until an action is found, which is executable.
This action returns a so-called `motion_command`, which is handed over to the `motion_selector` in [motion](../motion/overview.md).

## LED Eyes Documentation

### Left Eye

Multiple different things in the following order:

1. Red: in top or bottom half of the eye, latest processed image is longer than 1 second ago of respectively top or bottom camera (vision cycler is stalled/crashed/restarting)
2. Yellow: Referee Ready or FreeKick pose detected
3. Purple: Referee Ready or FreeKick pose percepted this cycle
4. Green: Ball Percept this control cycle
5. Black

### Right Eye

Always based on current role:

- Blue: Defender, Midfielder
- Yellow: Keeper, ReplacementKeeper
- Black (off): Loser
- White: Searcher
- Red: Striker
- Turquoise: StrikerSupporter
