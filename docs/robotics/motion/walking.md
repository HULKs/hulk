The whole process to let the robot walk is organized in three steps.

1.  The step planner creates the planned step.
    This includes x, y, and rotation depending on the motion command.

    !!! warning

        The step planning is currently under development and will be redesigned by [Narcha](https://github.com/Narcha).

        At the time of writing, only individual steps are planned, but not the whole step sequence.
        This is subject to change.

2.  The walk manager uses the planned step and the motion command to create the walk command, which defines the walking mode, such as standing, walking, or others.

3.  The walking engine uses the walk command and computes the according motor comamands.

## Walking Engine

The core of the walking engine is based on an idea by [Bernhard Hengst](https://www.researchgate.net/profile/Bernhard-Hengst) from ([UNSW Sydney](https://www.unsw.edu.au/)), which is used by almost all RoboCup SPL teams.<br>
The idea is quite simple:

- There are two feet, a _support foot_ and a _swing foot_
- Move the swing foot forward with speed $x^2$
- Move the support foot backward with speed $x$.

But around this, there's a lot of state handling and transitions.

!!! todo

    Add a diagram of the walking engine

## Return Offset and Ground Frame Compensation

In the HULKs walking engine, steps are planned relative to a dynamic coordinate system called Ground, which lies between the robot's feet.
This frame represents the robot's effective position on the field and is used by high-level behavior to request movement.

### Step Planning and Execution

The walking engine interpolates the robot's foot positions over time during each step. When a step ends - i.e., when the swing foot contacts the ground and becomes the support foot - a new step is planned. At every control cycle, the step planner assumes the current step will complete within that cycle and generates a plan for the next step accordingly.

### The Return Offset

After a non-zero movement step, the robot's feet are no longer side-by-side. To halt walking cleanly, the robot must place the swing foot next to the support foot, bringing the feet back to a resting position. This final adjustment moving the feet together inevitably shifts the Ground frame. The return offset is this shift: the isometric transformation (rotation and translation in 2D) of the Ground frame that occurs even when planning a nominal zero step.

For example, if the last step was 4 cm forward, the feet end 4 cm apart. To come to rest, the swing foot (now 2 cm behind the torso) must move forward 4 cm to align with the support foot. This results in a 2 cm forward movement of the Ground frame, even though the walking engine executes a 0 cm step. This movement must be compensated in planning.

### Why Compensation Is Necessary

All behavior-level movement requests are relative to the Ground frame. If the return offset is not accounted for, actual movement will differ from intended movement. For example, if behavior requests a 10 cm forward move, but the return offset will already advance the robot 2 cm, the step planner must only request an 8 cm step to achieve the intended net movement.

Similarly, actions like kicking, which depend on the position of the support foot relative to the ball, must consider where the Ground frame will be after the current step ends. This ensures correct timing and positioning for actions relative to other elements in the environment.

### Summary

- Ground: Coordinate system used by behavior, located between the robot's feet.
- Return Offset: The movement of the Ground frame due to feet realignment at the end of walking.
- Step planner adjusts requested steps to account for the return offset, ensuring behavior-level commands result in correct physical displacement of Ground.
- Planning always assumes the current step completes in the current cycle to provide walking with the next step to execute.
