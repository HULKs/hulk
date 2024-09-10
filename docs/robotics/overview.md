# Overview

Currently there are 6 different [threads](<https://en.wikipedia.org/wiki/Thread_(computing)>), called [cyclers](../framework/cyclers.md) in HULKs terminology, which are:

## Control

This cyler is responsible for the high-level control of the robot.
This includes [behavior](behavior/overview.md)-related tasks like selecting the action to execute next and [motion](motion/overview.md)-related tasks like executing the selected action such as dribbling, kicking, standing up and others. <br>
This cycler runs with higher priority than the other cyclers and with a higher frequency of 83 Hz, i.e. every 12ms.

!!! tip

    For more insights, open the [code](https://github.com/hulks/hulk) and have a look at the behavior and motion folders in the `control` crate. Follow the documentation here and in the code in parallel.

## VisionTop & VisionBottom

These two cyclers handle all image related tasks for top and bottom camera.
This includes the image segmenter, ball detection, line detection and other nodes. <br>
Both cyclers run with the frequency of the cameras which is 30 Hz.

## Audio

This cycler is responsible for audio processing.
It includes currently only one node, the whistle detection.

## SPLNetwork

This cycler handles all spl network messages, i.e. it is responsible for the communication with the GameController and other robots.

## ObjectDetectionTop

This cycler runs the pose detection of the referee.
