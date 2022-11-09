# Partitioning

The Nao uses a single flash storage device for main storage purposes.
After a successfully flashing the robot with the HULKs OS, this storage device is recogniced as `/dev/mmcblk1`.
The device counts 4 separate partitions.

```sh
NAME          SIZE MOUNTPOINTS
mmcblk1      29.1G
|-mmcblk1p1   128M /media/internal
|-mmcblk1p2    64M
|-mmcblk1p3   3.8G /
`-mmcblk1p4  25.1G /data
```

## Softbank partition `mmcblk1p1`

The first partition is called 'internal' and is used by softbank binaries and during the flashing.
The partition is mounted by default at `/media/internal` and is required to be mounted when using LoLA or HAL.
During standard use, this partition is not accessed by HULKs binaries.

Softbank uses this partition to store general information about the robot, such as IDs.
The aldebaran script `/opt/aldebaran/head_id` for example uses the file `/media/internal/DeviceHeadInternalGeode.xml` to query the id of the head.


## EFI partition `mmcblk1p2`

The second partition is the EFI boot partition and not mounted by default.
To inspect the EFI files mount this partition:

```sh
sudo su
# enter the password for the nao user
mount /dev/mmcblk1p2 /mnt/
# inspect files at /mnt/
```

## Root partition `mmcblk1p3`

The third partition is the root partition.
This partition is created and managed by the yocto configuration and usually not inteded to be modified at runtime.

## Data partition `mmcblk1p4`

The fourth and last partition is for runtime data storage.
It is mounted to `/data` by default.

When first booting up the system, the two system units `data-format` and `data-skeleton` are responsible of setting up the partition and directory structure.
The `data-format` unit is run once before mounting the partition to create a new filesystem and disables itself afterwards.
The `data-skeleton` unit is run every startup and provides a directory structure for following overlay mounts.

### Home directory `/home/nao`

The home directory is used for custom user code and also for storing and executing the `hulk` binary.
It is an overlay mount specified in the `/etc/fstab`:

```sh
[...]
overlay /home/nao overlay lowerdir=/home/nao,upperdir=/data/home/nao,workdir=/data/.work-home-nao 0 0
[...]
```

