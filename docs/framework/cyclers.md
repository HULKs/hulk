# Cyclers

A cycler in the HULKs robotic control software is a subcomponent that _cycles_ nodes.
The name "cycler" comes from the characteristic that it contains a loop that iterates over incoming data and produces output data in each iteration.
The cyclers call their internal `cycle()` function in each iteration.
This `cycle()` function consists of three steps:

1. _Setup_: Wait for new data and prepare cycle
2. _Process_: Run nodes on the received data
3. _Finalize_: E.g. send actuator commands or store data before starting the next cycle

Multiple cyclers exist in the whole robotic control software.
One of the main tasks of the framework is to allow cyclers to communicate with each other.
For example, in the _setup_ step, data from other cyclers and communication is gathered.
In addition, during the _finalize_ step, data produced in the _process_ step of this cycle may need to be communicated back to other cyclers.

Cyclers are separated into _realtime_ cyclers, e.g. the control cycler, and _perception_ cyclers, e.g. the vision cycler.

## Realtime Cyclers

A realtime cycler is a central cycler that has realtime characteristics.
It reacts to external events from the environment, then integrates data from the perception cyclers, and produces some output in the end.
One example is the control cycler which runs in realtime synchronized to the LoLA interval (83 Hz).
It receives sensor data from HULA/LoLA via the [Hardware Interface](./hardware_interface.md) and produces actuator output which is sent back to HULA/LoLA.
The control cycler integrates data from all other perception cyclers (e.g. audio, SPL network, vision) in its filtering pipeline.
Features for assisting in data integration in the filtering pipeline are explained in [Filtering](./filtering.md).
The control cycler contains all robotics code that needs to be evaluated in each realtime cycle.
In other words, all nodes that are required to generate new outputs are included.
Nodes that can be excluded or need to much computation, for example the vision pipeline, are executed to their own perception cyclers.

## Perception Cyclers

Beside the central realtime cyclers, multiple perception cyclers exist which perceive data from the outside world and preprocess it.
The outputs of each cycle are integrated in realtime cyclers to be respected for its realtime outputs.
Since perception cyclers run in parallel to realtime cyclers - and they are able to integrate historic data - perception cyclers may run at different cycle intervals.
Perception cyclers normally wait on an event triggered from outside e.g. a new camera image or network message.
The beginning of the processing is announced to realtime cyclers in the _setup_ step.
In addition, perception cyclers acquire requested data from the realtime cyclers.
The perception cycle's output data is sent to the realtime cyclers at the end of the cycle in the _finalize_ step.
More information about the interleaving of perceived data can be found in [Filtering](./filtering.md).
The following perception cyclers exist:

-   _audio_: Receives audio data from the [Hardware Interface](./hardware_interface.md) e.g. from NAO microphones
-   _spl_network_: Waits for incoming network messages or outgoing message sending requests from other cyclers.
    Each cycle either preprocesses the incoming messages (e.g. by parsing) or sends the outgoing messages to the network.
-   _vision_: Receives camera images from the [Hardware Interface](./hardware_interface.md) and processes them to extract several features.
