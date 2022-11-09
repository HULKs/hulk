# Home Directory

The home directory (`/home/nao/`) is overlayed with a mount to the `/data` partition (see [Partitioning](./partitioning.md)).

When flashing the robot and uploading the hulk binary, the home directory structure looks as follows:

```sh
.
|-- hulk
|   |-- bin
|   |   `-- hulk
|   |-- etc
|   |   |-- configuration
|   |   |   `-- *.json
|   |   |-- motions
|   |   |   `-- *.motion2
|   |   |-- neural_networks
|   |   |   `-- *.hdf5
|   |   |-- poses
|   |   |   `-- *.pose
|   |   `-- sounds
|   |       `-- *.ogg
|   `-- logs
|       |-- hulk-1667906932.err
|       |-- hulk-1667906932.out
|       |-- hulk.err -> /home/nao/hulk/logs/hulk-1667906932.err
|       `-- hulk.out -> /home/nao/hulk/logs/hulk-1667906932.out
`-- robocup.conf
```

The `./robocup.conf` file is required to start the LoLA service in robocupper mode.
All files related to the hulk service and binaries are stored in the subdirectory `hulk` and mirrors the files of the directory structure of the development repository ([Directory Structure](../framework/directory_structure.md)).
