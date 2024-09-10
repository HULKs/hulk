# Overview

!!! warning

    This structure in three steps is only conceptionally, in the code, there is no differentiation between these steps, and all nodes are automatically sorted during compile time depending on their inputs and outputs.

## Motion Selection

Motion starts in the `motion_selector` with the motion command from behavior.
Here the current motion is chosen based on the previous motion, if it is finished or if it can be aborted.

## Motion Execution

In the next step, all nodes for all motions are executed.
The nodes, whose motion is not selected, may exit early.

## Command Sending

Motion finishes by collecting all motor commands in the `motor_commands_collector`, sends them to the `motor_commands_optimizer`, and then writes them to the hardware interface in the `commands_sender`.
