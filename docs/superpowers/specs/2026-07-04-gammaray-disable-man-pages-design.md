# Gammaray Man-Page Generation Disable Design

## Issue

Issue #2795 reports that `pepsi gammaray` spends about 20 to 30 seconds generating man pages while installing packages on robots. The delay comes from Debian package installation side effects, not from the packages that `gammaray` actually needs.

## Goal

Make `pepsi gammaray` skip man-page installation and generation on target robots. This should reduce package-install time without changing the set of runtime packages that `gammaray` installs.

## Non-Goals

- Keep man-page support available on robots.
- Change the HULKs-OS image build.
- Refactor `gammaray` command execution or SSH handling.

## Chosen Approach

Add an early `gammaray` setup step that configures dpkg to exclude man-page paths on the target robot:

```sh
sudo mkdir -p /etc/dpkg/dpkg.cfg.d
printf 'path-exclude=/usr/share/man/*\n' | sudo tee /etc/dpkg/dpkg.cfg.d/01-disable-man-pages
sudo rm -rf /usr/share/man/* /var/cache/man/*
```

`man-db` registers triggers for man-page directories such as `/usr/share/man`. Excluding those paths prevents future package installs from unpacking man pages there, which avoids the expensive trigger work that `gammaray` does not need.

## Alternatives Considered

- Divert `mandb` to a no-op. This avoids the expensive command but still installs man pages and leaves dpkg trigger behavior active.
- Purge `man-db`. Future package installs can reintroduce it through dependencies, and package behavior may differ across robot images.
- Move the setting into the OS image. That is cleaner for future images but does not make `gammaray` self-contained.

## Design

Implement a small helper in `tools/pepsi/src/gammaray.rs`, for example `disable_man_pages(&Robot, &ProgressBar)`. The helper runs before `ADD_ZENOH_APT_SOURCES`, before the `zenohd`/`zenoh-bridge-dds`/`ufw` install, and before `install_podman`.

The helper should use the existing `ssh_with_log` path so progress reporting and error handling match the rest of `gammaray`. The remote command should be idempotent: repeated `gammaray` runs should leave the same dpkg config file in place and should not fail if the man-page directories are already empty.

`install-podman.sh` should remain unchanged. The new dpkg configuration applies before that script runs, so its `apt install --yes iptables uidmap util-linux` command benefits from the same setting.

## Error Handling

If the remote command fails, `gammaray` should fail before package installation. That failure is preferable to silently running the slow path because the command changes robot package-manager configuration.

## Testing

- Run `cargo fmt --package pepsi` after changing `gammaray.rs`.
- Run `cargo check -p pepsi` after implementation.
- On a robot or equivalent image, run `pepsi gammaray <NAO>` and verify package installation no longer spends time generating man pages.
- Check that `/etc/dpkg/dpkg.cfg.d/01-disable-man-pages` exists on the robot and contains `path-exclude=/usr/share/man/*`.

## Risks

Robots set up with `gammaray` will not have man pages installed for future packages. This is acceptable for issue #2795 because man-page support is not required on deployed robots.
