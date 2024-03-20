# Recording and Replay

The framework supports to record the robots data and replay it afterwards for easy analysis.
For each cycler instance, only the node states and inputs at the beginning of each cycle are recorded.
During replay, the inputs and node states are used to recompute all outputs.
A started communication server during replay can be used to investigate the recorded data via, e.g., Twix.

## Record

- Manual upload to a robot
    - Use `./pepsi recording ...` to enable recording for certain cycler instances, e.g., `./pepsi recording Control VisionTop`
        - This will set the cycler instances in `etc/parameters/framework.json`
    - Use `./pepsi upload ...` to upload as usual (this includes the framework configuration with the cycler instances)
- Pregame
    - Use `./pepsi pregame --cycler-instances-to-be-recorded ... ...` to enable recording for certain cyclers and upload to the robot in one step, e.g., `./pepsi pregame --cycler-instances-to-be-recorded Control,VisionTop ...`
        - This will set and overwrite the cycler instances in `etc/parameters/framework.json`

Be careful enabling vision cyclers because this will result in a lot of data being recorded. Top and bottom vision cyclers may fill the entire disk within approximately 10 minutes.

Data is only recorded during `PrimaryState::Ready`, `PrimaryState::Set`, and `PrimaryState::Play`.

## Replay(er)

Assuming you already recorded some data on a robot, you can now use the "replayer" tool to replay the recorded data.
First, you need to prepare the data.
The replayer assumes the recording files in `logs/<cycler_instance>.bincode`.
All cycler instance files need to be present, regardless whether they were enabled during recording (they will be empty then).

- Download the logs into a `logs` directory within the repository via, e.g., `./pepsi postgame ... logs ...`
- The `logs` directory contains directories per robot. Move all files of the robot that you want to replay into the `logs` directory.
- Rename all `*.bincode` files to remove the timestamp from the name to match the path `logs/<cycler_instance>.bincode`.
- Somehow™ obtain the head and body IDs of the robot that you want to replay
- Start the replayer tool via `./pepsi run --target replayer -- <BODY_ID> <HEAD_ID>`
- Connect your Twix to `localhost` and open some panels
- Move the slider to make data available to Twix. Pro Tip: Click into the text box and use your arrow keys to "animate".
- ...
- Profit
