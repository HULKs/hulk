# Overview

Apart from the robot code our repository contains several tools to aid in the development and testing process:

- [Pepsi](./pepsi.md): A multi-tool to automate repetitive tasks like compiling and deployment
- [Twix](./twix.md): The ROS-Z debugging UI
- [Remote Control](./remote_control.md): K1 gamepad controls and restoration of manufacturer control
- [Machine Learning](./machine-learning.md): Dataset annotation, Hydra/XFeat export, motion-policy training and TensorRT deployment
- [Simulator](./behavior_simulator.md): Experimental K1 behavior/motion development branch and worktree-only perception/tuning workflows
- [Behavior Tree Simulator Design](./behavior_tree_simulator_design.md): Design for simulating the current behavior tree directly
- [Debugging with GDB/LLDB](./debugging.md): How to use a debugger with our software
- [Profiling with `perf`](./profiling.md): How to profile our software with `perf`

!!! note "Simulator under development"

    The simulator is not available on main. Check out the [`motion-inference-simulator` branch in Alex's repository](https://github.com/alexschmander/hulk/tree/motion-inference-simulator) for its experimental implementation. Perception, tuning and optimization-monitor workflows additionally require matching experimental worktree additions, which are absent from the currently tracked remote branch. See the [simulator page](./behavior_simulator.md) for checkout instructions and availability.
