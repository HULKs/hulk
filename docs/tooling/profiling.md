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
Due to an automatic renaming during upload, your binary might be called `hulk_nao`. Hotspot expects it to match the process name (`hulk`), so rename it:

```bash
mv ./target/x86_64-aldebaran-linux-gnu/with-debug/{hulk_nao, hulk}
```

Run Hotspot

With the binary and `perf.data` in place, launch Hotspot:
```bash
hotspot --appPath ./target/x86_64-aldebaran-linux-gnu/with-debug/
```

Use the Flame Graph, Top Down, or Bottom Up views to investigate time spent in functions.

### Step 4: Inspect System and Kernel Code (Optional)

To inspect system libraries or kernel code, additional debug information is needed.
Export Kernel Symbol Addresses

To resolve kernel function names and addresses, export /proc/kallsyms:

```bash
sudo cat /proc/kallsyms > kallsyms
```

!!! warning
    You must run this as root; otherwise, addresses will be replaced with 00000000 and are unusable.

### Enable Debug Symbols in the SDK

To get debug information for system libraries, you need an SDK with debug symbols. Enable them by modifying this line in your Yocto distribution config:

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
Rebuild the SDK afterward.

### Step 5: Launch Hotspot with Full Debug Info

Once you have the debug SDK and kallsyms, you can launch Hotspot with full debug info:

```bash
hotspot \
  --sysroot ~/.local/share/hulk/sdk/9.0.2/sysroots/corei7-64-aldebaran-linux/ \
  --appPath ./target/x86_64-aldebaran-linux-gnu/with-debug/ \
  --kallsyms ./kallsyms
```
