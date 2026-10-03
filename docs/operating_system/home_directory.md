# Home Directory and Logs

Current deployment uses `/home/booster/hulk`. Pepsi connects as `booster` and
uploads the binary and the repository's `etc` directory into `hulk`.
The runtime bind-mounts `/home/booster`, with working directory
`/home/booster/hulk`.

The relevant layout is:

```text
/home/booster/
├── hulk/
│   ├── bin/hulk_ros_z
│   ├── etc/
│   │   ├── parameters/
│   │   │   ├── team.toml
│   │   │   ├── base/*.json5
│   │   │   ├── location/<location>/*.json5
│   │   │   └── robot/<hardware-id>/*.json5
│   │   ├── neural_networks/
│   │   └── sounds/
│   └── logs/
│       ├── source/
│       ├── <timestamp>/
│       │   ├── hulk.out
│       │   └── hulk.err
│       └── latest -> <timestamp>/
└── .cache/hulk/
    ├── tensor-rt/
    └── runtime-container-image.tar  (when supplied to gammaray)
```

`robot/<hardware-id>` overrides are created as needed. Models include ONNX and
TensorRT artifacts; download Git LFS assets before deployment.

`/usr/bin/launch-hulk` creates the timestamped log directory and `latest` link,
sets ownership to `booster`, and executes `hulk_ros_z` inside the container.
It passes `--parameter-root etc/parameters`, `--location default-location`,
`--router tcp/127.0.0.1:7447`, and `--log-path <timestamp-directory>`.
Additional recorder output can be written under that log path.

```sh
./pepsi log show <robot-IP>
./pepsi log list <robot-IP>
./pepsi log download <local-directory> <robot-IP>
./pepsi shell <robot-IP> "sudo journalctl -u hulk -u hulk-runtime --no-pager -n 100"
```

`log show` reads `logs/latest/hulk.{out,err}`. `log download` also captures kernel
and system journals. Download logs before a normal upload if you want to retain
them: upload cleans remote files by default; `--no-clean` changes that behavior.
The repository's current `log delete` implementation still targets
`/home/robot/hulk/logs/*`, so it is not a reliable cleanup command for this layout.
