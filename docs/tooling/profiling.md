# Profiling on the Nao Robot

We use the perf command to profile applications running on the Nao robot. It comes preinstalled in our robot image.

!!!warning
    Important: Ensure that your hulk binary includes debug symbols; without them, the profile will be unusable.
    Enable debug symbols by adding the --profile with-debug option to either the pepsi upload or pepsi build command. This preserves compiler optimizations while retaining symbol information for profiling.

### Step 1: SSH into the Robot

To profile an application running on the Nao, you must SSH into the robot first:

```bash
pepsi shell <nao-number>
```

### Step 2: Record a Profile with perf

To profile a running application (e.g., hulk), use:

```bash
perf record --call-graph dwarf,8192 --aio -z --sample-cpu --mmap-pages 16M --pid $(pidof hulk) sleep 30
```

This samples the stack traces of the hulk process for 30 seconds.

!!!info
    It's generally easiest to run perf as root, as otherwise various permissions and kernel knobs must be adjusted.

This command will generate a `perf.data` file containing the recorded samples.

### Step 3: Analyze the Profile with hotspot

We use Hotspot to inspect the `perf.data` file.
With the binary and `perf.data` in place, launch Hotspot:

```bash
hotspot \
  --sysroot ~/.local/share/hulk/sdk/9.0.2/sysroots/corei7-64-aldebaran-linux/ \
  --appPath ./target/x86_64-aldebaran-linux-gnu/with-debug/ \
  --kallsyms ./kallsyms
```

Setting the `sysroot` is not requires when you profile a binary on your system, for example a behavior test case.
Use the Flame Graph, Top Down, or Bottom Up views to investigate time spent in functions.

### Enable Debug Symbols in the SDK

By default, debug symbols are not enabled in our SDK.
If you want to profile library code, consider enabling debug symbols in the Yocto distribution config:

```
diff --git a/meta-hulks/conf/distro/HULKs-OS.conf b/meta-hulks/conf/distro/HULKs-OS.conf
index 3e23671..982a187 100644
--- a/meta-hulks/conf/distro/HULKs-OS.conf
+++ b/meta-hulks/conf/distro/HULKs-OS.conf
@@ -5,4 +5,3 @@ SUMMARY = "HULKs flavoured Nao"
 DISTRO = "HULKs-OS"
 DISTRO_NAME = "HULKs-OS"
 DISTRO_VERSION = "9.0.1"
-SDKIMAGE_FEATURES:remove = "dbg-pkgs src-pkgs"
```
Rebuild the SDK afterward and use in it to build the code and as the `sysroot` argument when launching Hotspot.
