# Linux on the Booster

The current stack targets the Booster K1's AArch64 Jetson Linux environment.
The repository does not define or build the robot's kernel. The vendor release
expected by upload is recorded in root `hulk.toml`, and the runtime image base
is defined in `tools/k1-setup/inference-runtime/Containerfile`.

Inspect the actual robot when kernel or device compatibility matters:

```sh
./pepsi shell <robot-IP> "uname -a"
./pepsi shell <robot-IP> "cat /opt/booster/version.txt"
./pepsi shell <robot-IP> "jetson_release -s"
```

Gammaray installs the repository's services and runtime configuration on an
already working vendor OS. It configures NVIDIA CDI, Podman, Zenoh services,
and Jetson clock refresh; see [Booster setup](../setup/booster_setup.md).
Application/runtime lifecycle is managed through systemd:

```sh
./pepsi hulk status <robot-IP>
./pepsi shell <robot-IP> "sudo systemctl status hulk-runtime"
```
