# Build GitHub Actions Runner v3 for Proxmox LXC

## Build root filesystem

- Install [distrobuilder](https://linuxcontainers.org/distrobuilder/introduction/)
- Build via `distrobuilder build-lxc --compression zstd v3.yaml`
- The root filesystem is named `rootfs.tar.zst`

## Upload root filesystem to Proxmox

- Open the storage that supports uploading "CT Templates"
- "Upload" the `root.tar.zst` and set a reasonable name following the naming convention already existing

## Create a new container with the root filesystem

- "Create CT"
    - Select node with fast IO device
    - Let yourself allocate an IP address (maybe ask someone that can help you)
    - Container ID will be last octet of the IP address
    - Select a hostname (TODO: hostname naming convention?)
    - Unselect "Unprivileged container" because container nesting requires this
    - "Resource Pool" will stay as is
    - Select password and store that in our vault
    - Enter at least your SSH key (we will add more later)
    - Other default settings for GitHub Actions Runners
        - 512 GiB storage
        - 32 CPU cores
        - 65536 MiB RAM and 0 MiB swap
    - Create the container
- Go to "Options" and enable "Start at boot" and enable "Nesting" under "Features"
- Start the container
- SSH via root to the container
    - `mkdir /home/hulk/.ssh && cp .ssh/authorized_keys /home/hulk/.ssh/ && chown -R hulk:hulk /home/hulk/.ssh`
    - Set `hulk` user password via `passwd hulk` and store that in our vault
- Visit https://github.com/actions/runner/releases/latest and find the asset `actions-runner-linux-x64-*.tar.gz` and copy the link
- SSH via hulk to the container
    - Download the runner archive e.g. via `wget`
    - `mkdir actions-runner && cd actions-runner && tar xzf ../actions-runner-linux-x64-*.tar.gz`
    - Execute `./config.sh ...` from GitHub to add a new runner (add `v3` label)
    - Execute `sudo ./svc.sh install && sudo ./svc.sh start`
