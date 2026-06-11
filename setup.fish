#!/usr/bin/env fish

set -l iterations $argv[1]
if test -z "$iterations"; or not string match -qr '^[0-9]+$' -- "$iterations"; or test "$iterations" -lt 1
    printf 'Usage: %s <iterations>\n' (status filename)
    exit 1
end

mkdir -p ./instances
for i in (seq -w 1 $iterations)
       cp -r workshop/ instances/$i
end
