preserve_booster_config() {
    local boot updated
    boot="$(readlink -f "$1")"
    updated="$(mktemp "${boot}.XXXXXX")"
    cp --preserve=mode,ownership "$boot" "$updated"

    if ! python3 - "$boot" "$updated" <<'PY'
import pathlib
import sys

boot = pathlib.Path(sys.argv[1])
bootstrap = """    rm -f /etc/booster/config/containerd/config.yaml
    reload_config_template
    apply_platform_config
"""
launch = "\n/opt/booster/bin/containerd \\\n"
guarded_launch = """
# HULKs: check before containerd can create a default runtime config.
hulks_initialize_config=false
if [[ ! -s /etc/booster/config/containerd/config.yaml ]]; then
    hulks_initialize_config=true
fi
""" + launch
call = "if ! apply_master_config_if_needed; then\n"
guarded_call = 'if [[ "${hulks_initialize_config}" == true ]] && ! apply_master_config_if_needed; then\n'
source = boot.read_text()
if source.count(bootstrap) != 1 or source.count(launch) != 1:
    sys.exit(f"Unsupported Booster startup script: {boot}")
if source.count(call) == 1 and guarded_launch not in source and guarded_call not in source:
    source = source.replace(launch, guarded_launch).replace(call, guarded_call)
elif source.count(guarded_launch) != 1 or source.count(guarded_call) != 1 or call in source:
    sys.exit(f"Unsupported Booster startup script: {boot}")
pathlib.Path(sys.argv[2]).write_text(source)
PY
    then
        rm -f "$updated"
        return 1
    fi

    if ! bash -n "$updated"; then
        rm -f "$updated"
        return 1
    fi
    if cmp -s "$boot" "$updated"; then
        rm -f "$updated"
    else
        mv "$updated" "$boot"
    fi
}
