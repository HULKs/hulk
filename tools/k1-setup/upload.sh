#!/usr/bin/env sh

rsync -av --rsync-path='sudo rsync' tools/k1-setup/hulk tools/k1-setup/launchHULK booster@192.168.10.102:/bin/
rsync -av --rsync-path='sudo rsync' tools/k1-setup/hulk.service tools/k1-setup/zenoh-bridge.service booster@192.168.10.102:~/.config/systemd/user/
