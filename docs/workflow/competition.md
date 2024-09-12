The HULKs visit various competitions regularly, such as the [RoboCup German Open](https://robocup.de/german-open/) or the international [RoboCup](https://www.robocup.org/).
During these events, good organization and communication are key to success, a good workflow and not too much stress.

## Meetings

During periods of high activity, a daily Standup meeting is usually held to inform each other about the current status and to coordinate the next steps.

This meeting (as well as all other important events) is documented in the Nextcloud calendar.
You should have access to it, if not, ask somebody from the team.
Ideally, you should also have it synced to your phone.

## Roles

To distribute tasks and responsibilities, different roles are assigned to the team members.
Usually, the roles are assigned during the pregame meeting, only the role of the Head-of-Robots (HoR) is assigned for the whole competition.

!!! note

    During the game, only the Team Captain, Deployer, and Logführer are allowed to stand next to the Game Controller.

### Team Captain

Is usually one of the Dev-Leads, is responsible for the organization of the meetings and the overall schedule.
Decides which code is deployed.

### Deployer

Merge-squashes different branches and deploys the code to the robots.

??? note "Requirements"

    The deployer should have the necessary hardware or setup for fast deployment.
    I.e. a fast laptop or a working remote build setup.
    Strong nerves are also a plus.

### Logführer

Is responsible during the game for observing the game and noting down important events and things that need to be improved.

### Head-of-Robots (HoR)

Is responsible for the robots and the hardware, as well as all interactions with [URG](https://unitedrobotics.group/en/robots/nao).
Keeps the [Roboboboard](https://github.com/orgs/HULKs/projects/3) up to date and selects the robots for the game, as well as their number.

!!! tip "Important"

    This role is extremely important, as the hardware status (especially in the later games) is crucial for the performance.
    Also, having a good relationship with URG can be beneficial for the team.

## Game Schedule

For games, a strict schedule is created, which looks like this:

-   90 minutes prior: Pregrame Meeting <br>
    Here, the roles are assigned and last steps and important tasks before the game are discussed.
-   45 minutes prior: Code at deployer <br>
    All branches are ready to be merged and deployed.
    Sometimes parameter changes are still made after this stage.
-   30 minutes prior: Golden Goal <br>
    A kick-off against an empty field is performed. This is the final test before the game.
