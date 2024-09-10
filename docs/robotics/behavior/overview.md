# Overview

The behavior of the robot is mainly controlled in the `node` node.
Here it is decided which actions we would like to take.
This is organized in two steps:

## Action Collection

In this step, the world state and other inputs are used to create a priority sorted list of actions, which we would like to take.

-   Examples: `Unstiff`, `SitDown`, `Penalize`, `Initial`, `FallSafely`, `StandUp`, ...

Depending on the current situation, different actions are added to the list of available actions.

-   Example: If the current role is keeper and a penalty shootout is happening, the actions `Jump` and `PrepareJump` are added.

## Action Selection

Now, the list of actions is iterated until an action is found, which is executable.
This action returns a so-called `motion_command`, which is handed over to the `motion_selector` in [motion](../motion/overview.md).
