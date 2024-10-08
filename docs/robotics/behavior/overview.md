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
