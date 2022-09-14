# Overview

TODO: Mention unit testing

This section explains the framework of our NAO software.
The chapters walk through various features in a top-down approach starting with a general overview.
More advanced topics are covered later.
Here is a short outline of the next chapters:

- [Directory Structure](./directory_structure.md): Explains the directory structure of the code repository
- [Process EntryPoint](./process_entrypoint.md): Starts the top-down approach from the `main()` function of the process
- [Runtime](./runtime.md): What does the runtime do to setup and inter-connect all subcomponents?
- [Cyclers](./cyclers.md): How do cyclers run the robotics modules?
- [Modules](./modules.md): What are modules and how are they implemented?
- [Databases & Types](./databases_and_types.md): How can data be shared between cyclers and the framework?
- [Configuration](./configuration.md): How does the framework provide configuration parameters to modules?
- [Communication](./communication.md): What is communication and how is it able to communicate between framework and modules?
- [Hardware Interface](./hardware_interface.md): How is the hardware abstracted away for the different target platforms?
- [Thread Communication](./thread_communication.md): Which concepts and features exist to enable thread-safe communication between subcomponents?
- [Filtering](./filtering.md): How to interleave historic data in filters in an multi-threaded software?
- [Macros](./macros.md): What macros exist that ease the development and how do they work?
- [Error Handling](./error_handling.md): Which kinds of error handling concepts are supported and which to choose when?

The framework provides the fundamentals needed to execute robotics specific code.
It has a modular design to allow for convenient development and replacement of individual modules.
The framework consists of four fundamental components:

- [Runtime](./runtime.md): Encapsulates all subcomponents by starting and initializing them
- [Hardware Interface](./hardware_interface.md): Abstracts hardware away and is the interaction point for cyclers with the outside world
- [Cyclers](./cyclers.md): Cycle through modules, process data from hardware and produce outputs (see e.g. _control_ or _vision_top_)
- [Communication](./communication.md): Exchanges data between framework and other resources e.g. file system and network

![overview](./overview.drawio.png)
