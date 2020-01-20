#!/bin/sh

/usr/libexec/reset-cameras.sh toggle
echo $(/bin/date) Camera reset! >> /home/nao/naoqi/camera-reset.log
sleep 3
