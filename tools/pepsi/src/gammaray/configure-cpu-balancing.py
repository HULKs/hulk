import os
from pathlib import Path
import re
import shutil
import sys
import tempfile


MOTION_CPUS = {4, 5}


def cpu_list(value):
    cpus = set()
    value = value.strip()
    if not value:
        return cpus
    for part in value.split(","):
        if not re.fullmatch(r"\d+(?:-\d+)?", part):
            raise ValueError(f"unsupported CPU list: {value}")
        first, _, last = part.partition("-")
        first, last = int(first), int(last or first)
        if last < first:
            raise ValueError(f"descending CPU range: {part}")
        cpus.update(range(first, last + 1))
    return cpus


def balance_argument(match):
    parts = match[1].split(",")
    flags = []
    while parts and parts[0] in {"domain", "nohz", "managed_irq"}:
        flags.append(parts.pop(0))
    if flags and "domain" not in flags:
        return match[0]
    cpus = cpu_list(",".join(parts))
    remaining = cpus - MOTION_CPUS
    if remaining == cpus:
        return match[0]
    other_flags = [flag for flag in flags if flag != "domain"]
    if remaining and other_flags:
        raise ValueError("cannot split mixed isolcpus flags across motion and other CPUs")
    if remaining:
        return "isolcpus=" + ",".join(map(str, sorted(remaining)))
    if other_flags:
        return "isolcpus=" + ",".join(other_flags + parts)
    return ""


def configure(boot):
    original = boot.read_text()
    default = re.search(r"(?m)^\s*DEFAULT[ \t]+(\S+)", original)
    if default is None:
        raise ValueError(f"missing DEFAULT boot entry in {boot}")
    selected = False
    found_append = False
    lines = []
    for line in original.splitlines(keepends=True):
        label = re.match(r"\s*LABEL[ \t]+(\S+)", line)
        if label:
            selected = label[1] == default[1]
        if selected and re.match(r"\s*APPEND[ \t]", line):
            found_append = True
            line = re.sub(r"(?<!\S)isolcpus=(\S+)", balance_argument, line)
        lines.append(line)
    if not found_append:
        raise ValueError(f"missing APPEND in default boot entry {default[1]}")
    updated = "".join(lines)
    if updated == original:
        return

    descriptor, name = tempfile.mkstemp(dir=boot.parent, prefix=boot.name + ".")
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "w") as file:
            file.write(updated)
            file.flush()
            shutil.copymode(boot, temporary)
            os.fsync(file.fileno())
        os.replace(temporary, boot)
        directory = os.open(boot.parent, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        temporary.unlink(missing_ok=True)


if __name__ == "__main__":
    configure(Path(sys.argv[1]))
    isolated = cpu_list(Path("/sys/devices/system/cpu/isolated").read_text())
    print("reboot-required" if isolated & MOTION_CPUS else "ready")
