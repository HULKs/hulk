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

The core of the walking engine is based on an idea by Bernhard Hengst from (UNSW Sydney), which is used by almost all RoboCup SPL teams.<br>
The idea is quite simple:

-   There are two feet, a _support foot_ and a _swing foot_
-   Move the swing foot forward with speed $x^2$
-   Move the support foot backward with speed $x$.

But around this, there's a lot of state handling and transitions.

!!! todo

    Add a diagram of the walking engine
