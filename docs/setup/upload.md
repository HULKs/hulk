# Uploading HULK

To upload to a Booster robot, first [set up development](development_environment.md) and provision the robot's [runtime](../tooling/pepsi.md#offline-runtime-installation), then run:

```sh
./pepsi upload <robot>
```

Pepsi prepares the pinned robot SDK with Nix on demand.
To warm the cache without building HULK, run `./pepsi sdk prepare`.
A green check mark next to a robot indicates a successful upload.

!!! tip

    Continue reading about [our workflow and how to contribute](../workflow/overview.md).
