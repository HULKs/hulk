#!/usr/bin/env fish

set -l iterations $argv[1]
set -l notebook_kind $argv[2]
if test -z "$iterations"; or not string match -qr '^[0-9]+$' -- "$iterations"; or test "$iterations" -lt 1
    printf 'Usage: %s <iterations> <workshop|intro>\n' (status filename)
    exit 1
end
switch "$notebook_kind"
    case workshop
        set notebook louisenlund_workshop_2026.py
    case intro
        set notebook pythonIntro.py
    case '*'
        printf 'Usage: %s <iterations> <workshop|intro>\n' (status filename)
        exit 1
end

for i in (seq -w 1 $iterations)
    set -l port (math 2000 + $i)
    podman run \
        --replace \
        --rm \
        --detach \
        --name lund-workshop-$i \
        -v ./instances/$i:/workshop \
        --workdir /workshop \
        -p $port:$port \
        -e MUJOCO_GL=egl \
        -e PYOPENGL_PLATFORM=egl \
        workshop-uv \
        uv run marimo edit $notebook \
            --host 0.0.0.0 \
            --port $port \
            --token-password hulksworkshop
end
