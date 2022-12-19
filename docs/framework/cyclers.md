# Cyclers

A cycler in the HULKs robotic control software is a subcomponent that *cycles* nodes.
The name "cycler" comes from the characteristic that it contains a loop that iterates over incoming data and produces output data in each iteration.
The cyclers call their internal `cycle()` function in each iteration.
This `cycle()` function consists of three steps:

1. *Prepare*: Wait for new data and prepare cycle
2. *Process*: Run nodes on the received data
3. *Finalize*: E.g. send actuator commands or store data before starting the next cycle

Multiple cyclers exist in the whole robotic control software. One of the main tasks of the framework is to allow cyclers to communicate with each other.
For example, in the *prepare* step, data from other cyclers and communication is gathered.
In addition, during the *finalize* step, data produced in the *process* step of this cycle may need to be communicated back to other cyclers.

Cyclers are separated into the control cycler and multiple perception cyclers e.g. the vision cycler.

## Control Cycler

The control cycler is the central cycler that runs in realtime synchronized to the LoLA interval (83 Hz).
It receives sensor data from HULA/LoLA via the [Hardware Interface](./hardware_interface.md) and produces actuator output which is sent back to HULA/LoLA.
The control cycler integrates data from all other perception cyclers in the filtering pipeline.
Features for assisting in data integration in the filtering pipeline are explained in [Filtering](./filtering.md).
The control cycler contains all robotics code that needs to be evaluated in each realtime cycle.
In other words, all nodes that are required to generate new outputs are included.
Nodes that can be excluded or need to much computation, for example the vision pipeline, are executed in their own perception cyclers.

## Perception Cyclers

Beside the central control cycler, multiple perception cyclers exist which perceive data from the outside world and preprocess it.
The outputs of each cycle are integrated in the control cycler to be respected for its realtime outputs.
Since perception cyclers run in parallel to the control cycler and the control cycler is able to integrate historic data, perception cyclers may run at different cycle intervals.
Perception cyclers normally wait on an event triggered from outside e.g. a new camera image or network message.
The beginning of the processing is announced to the control cycler in the *prepare* step.
In addition, perception cyclers acquire requested data from the control cycler.
The perception cycle's output data is sent to the control cycler at the end of the cycle in the *finalize* step.
More information about the interleaving of perceived data can be found in [Filtering](./filtering.md).
The following perception cyclers exist:

- *audio*: Receives audio data from the [Hardware Interface](./hardware_interface.md) e.g. from NAO microphones
- *spl_network*: Waits for incoming network messages or outgoing message sending requests from other cyclers.
  Each cycle either preprocesses the incoming messages (e.g. by parsing) or sends the outgoing messages to the network.
- *vision_top*: Receives top camera images from the [Hardware Interface](./hardware_interface.md) and processes them to extract several features.
- *vision_bottom*: Similar to *vision_top* but receives camera images from the bottom camera.
