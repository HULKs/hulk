set -euo pipefail

mode="$1"
case "$mode" in
    enable|disable) ;;
    *) echo "Expected enable or disable" >&2; exit 1 ;;
esac

preserve_booster_config /opt/booster/bin/script/boot
source /opt/booster/env/containerd.env
bdb=(/opt/booster/bin/bdb --port "$BOOSTER_CONTAINERD_GATEWAY_PORT")

wait_for_config() {
    local deadline=$((SECONDS + 30))
    until [[ -s /etc/booster/config/containerd/config.yaml ]] &&
        "${bdb[@]}" config status --json 2>/dev/null |
        python3 -c 'import json, sys; s = json.load(sys.stdin)["status"]; sys.exit(not (s["active"] and s["report"]["valid"]))' 2>/dev/null &&
        "${bdb[@]}" service status robot_core --json >/dev/null 2>&1
    do
        if (( SECONDS >= deadline )); then
            echo "Booster config did not become ready within 30 seconds" >&2
            return 1
        fi
        sleep 1
    done
}

# Run the patched startup before editing the draft, including on first setup.
# Boosterize deliberately requests fresh platform defaults.
systemctl stop hulk
if [[ "$mode" == enable ]]; then
    systemctl stop booster-daemon
    if [[ -f /etc/booster/config/containerd/config.yaml ]]; then
        cp --preserve=mode,ownership /etc/booster/config/containerd/config.yaml /etc/booster/config/containerd/config.yaml.before-boosterize
        rm /etc/booster/config/containerd/config.yaml
    fi
fi
systemctl restart booster-daemon
wait_for_config

if [[ "$mode" == disable ]]; then
    services=$("${bdb[@]}" config get services --json)
    disabled=$(python3 -c '
import json, sys
services = json.load(sys.stdin)["services"]
names = {s["name"] for s in services}
required = {"remote_controller", "ros.agent_manager", "battery_daemon", "server.motion", "ros.hand_ros2_bridge_left", "ros.hand_ros2_bridge_right"}
if not required <= names:
    sys.exit("Missing Booster services: " + ", ".join(sorted(required - names)))
for service in services:
    name = service["name"]
    if (service["container"] == "perception" and name != "battery_daemon") or name in required - {"battery_daemon", "server.motion"} or "x5" in name.lower():
        print(name)
' <<< "$services")
    while IFS= read -r service; do
        "${bdb[@]}" config set service "$service" enable false
    done <<< "$disabled"
    "${bdb[@]}" config set service battery_daemon deps '[]'
    "${bdb[@]}" config set service server.motion deps '[]'
    "${bdb[@]}" config validate
    "${bdb[@]}" config apply

    # Applying saves the config; containerd rebuilds its live service list on restart.
    systemctl restart booster-daemon
    wait_for_config
fi
systemctl "$mode" --now joystick_ros2
