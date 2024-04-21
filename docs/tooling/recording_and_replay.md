# Recording and Replay

The framework supports to record the robots data and replay it afterwards for easy analysis.
For each cycler instance, only the node states and inputs at the beginning of each cycle are recorded.
During replay, the inputs and node states are used to recompute all outputs.
A started communication server during replay can be used to investigate the recorded data via, e.g., Twix.

## Record

- Manual upload to a robot
    - Use `./pepsi recording ...` to enable recording at different recording rates, e.g., `./pepsi recording Control=1,VisionTop=30`
        - This will set the cycler instances and recording rate in `etc/parameters/framework.json`
    - Use `./pepsi upload ...` to upload as usual (this includes the framework configuration with the cycler instances)
- Pregame
    - Use `./pepsi pregame --recording-intervals ... ...` to enable recording at different recording rates and upload to the robot in one step, e.g., `./pepsi pregame --recording-intervals Control=1,VisionTop=30 ...`
        - This will set and overwrite the recording intervals in `etc/parameters/framework.json`

Be careful enabling vision cyclers because this will result in a lot of data being recorded. Top and bottom vision cyclers may fill the entire disk within approximately 10 minutes.

Data is only recorded during `PrimaryState::Ready`, `PrimaryState::Set`, and `PrimaryState::Play`.

## Replay(er)

Assuming you already recorded some data on a robot, you can now use the "replayer" tool to replay the recorded data.

- Download the logs into a `logs` directory within the repository via, e.g., `./pepsi postgame ... my_awesome_replay ...`
- The `my_awesome_replay` directory now contains directories for each robot. Each robot directory contains one directory with the replay data from one execution of the `hulk` binary.
  All cycler instance files need to be present, regardless whether they were enabled during recording (they will be empty then).
- Start the replayer tool by pointing it to the log directory you want to replay, e.g., `./pepsi run --target replayer -- my_awesome_replay/10.1.24.42/12345678`.
- Connect your Twix to `localhost` and open some panels
- Use mouse and keyboard in replayer, as described below
- ...
- Profit

### Mouse and Keyboard Controls

- Mouse dragging: Move the current replay time position (green bar)
- Horizontal scrolling: Panning in time domain
- Vertical scrolling: Zooming in time domain
- Horizontal scrolling with pressed Shift key: Panning in time domain
- Pressing J or down arrow key: jump 10 seconds backward
- Pressing L or up arrow key: jump 10 seconds forward
- Pressing left arrow key: jump 1 second backward
- Pressing right arrow key: jump 1 second forward
- Pressing comma key: jump 10 milliseconds backward
- Pressing dot key: jump 10 milliseconds forward

## Image extraction

To extract images from recording data, you can use the "imagine" tool.

Example:
```
./pepsi run --target imagine -- my_awesome_replay/10.1.24.42/12345678` `path/to/output`
```
