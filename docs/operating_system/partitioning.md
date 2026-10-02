# Storage and Partitioning

Booster storage layout is supplied by the vendor OS; this repository's
provisioning does not define a partition table or format a data partition.
Device names and capacities depend on the robot installation. Inspect them
before performing storage maintenance:

```sh
./pepsi shell <robot-IP> "lsblk -f"
./pepsi shell <robot-IP> "findmnt -T /home/booster"
./pepsi shell <robot-IP> "df -h /home/booster"
```

HULK deploys into `/home/booster/hulk`; its runtime bind-mounts `/home/booster`
rather than using the documented legacy NAO `/data` overlay.
See [Home Directory](home_directory.md) for binaries, model assets, caches, and logs.
The runtime image itself is stored in rootful Podman's image store on the robot.

The old NAO four-partition example, EFI partition, and home overlay are preserved
in [Historical: Partitioning](../historical/operating_system/partitioning.md).
Those device names, sizes, and first-boot units are image-specific legacy reference.
