# Parameters

The robotic control software has some configuration parameters that affect calculations and the execution of the code.
Modules can access deep fields in the hierarchy via a path.

## Loading and Modifications

The parameters are loaded from the file system.
The parameter files are located in `etc/parameters/`.
This directory also gets deployed to the NAO s.t. the `hulk` executable has access to it.
[Communication](./communication.md) is able to mutate parameter values at runtime (but cannot store them back to files).

## Overwriting

The parameter directory allows to overwrite individual configuration fields in the object hierarchy.
NAO robots have their cameras and mainboard in the head.
The chestboard and motors are attached to the body.
Each head and body of a NAO have unique IDs which are used to load specific configuration parameters.
In addition, robots may need different parameters depending on the location.
The location is selected in the parameter directory and points to a directory where overwriting parameter files are placed.
To create a full parameter object from the whole parameter directory, the following procedure is used:

1. Read and parse `etc/configuration/default.json`
2. If existing, read and parse...
    - For NAO: `etc/nao_location/default.json`
    - For Webots: `etc/webots_location/default.json`
    - For behavior simulator: `etc/simulated_behavior/default.json`
3. If existing, read and parse `etc/body.{body_id}.json`
4. If existing, read and parse `etc/head.{body_id}.json`
5. If existing, read and parse...
    - For NAO: `etc/nao_location/body.{body_id}.json`
    - For Webots: `etc/webots_location/body.{body_id}.json`
    - For behavior simulator: `etc/simulated_behavior/body.{body_id}.json`
6. If existing, read and parse...
    - For NAO: `etc/nao_location/head.{body_id}.json`
    - For Webots: `etc/webots_location/head.{body_id}.json`
    - For behavior simulator: `etc/simulated_behavior/head.{body_id}.json`

The location directories are usually symlinks to actual directories with the location names.
This allows to easily swap locations by retargeting the symlink.
