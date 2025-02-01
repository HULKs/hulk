#! /usr/bin/env sh

rsync -rP --info=progress2 --exclude-from=.gitignore --exclude=.venv . $1
