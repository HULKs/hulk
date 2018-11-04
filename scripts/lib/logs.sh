#!/bin/bash 

source "${BASEDIR}/scripts/lib/naonet.sh"

function delete_logs {
    BASEDIR="$1"
    RSYNC_TARGET="$2"
    ERROR=0

    msg -n "Cleaning up USB..."
    naocmd $BASEDIR $RSYNC_TARGET "rm -vfr \
        /mnt/usb/replay_* \
        /mnt/usb/filetransport_* \
        /home/nao/naoqi/filetransport_* \
        /home/nao/naoqi/replay_* \
        /home/nao/naoqi/tuhhNao.*
        "
    naocmd $BASEDIR $RSYNC_TARGET "ls -d /mnt/usb/filetransport_*"
    RETURN_CODE_1=$?
    naocmd $BASEDIR $RSYNC_TARGET "ls -d /mnt/usb/replay_*"
    RETURN_CODE_2=$?
    if [ "$RETURN_CODE_1" -ne "0" ] && [ "$RETURN_CODE_2" -ne "0" ]; then
        msg -n "[done]"
    else
        msg -e "Failed to clean up USB of ${RSYNC_TARGET}!"
        ERROR=1
    fi

    return $ERROR
}

function download_logs {
    BASEDIR="$1"
    NAO="$2"
    LOGDIR="$3"
    ERROR=0

    # download filetransport/replay
    mkdir -p "${LOGDIR}/${NAO}"
    msg -n "Downloading logs from ${NAO} into '$LOGDIR'"

    # download stdout/stderr
    msg -n "Downloading std{out|err}..."
    naocp "${BASEDIR}" \
        "nao@${NAO}:naoqi/tuhhNao.*" \
        "${LOGDIR}/${NAO}"
    if [ "$?" -ne 0 ]; then
      msg -e "Failed to download service logs from ${NAO}!"
    else
      msg -n "[done]"
    fi

    # download filetransport from usb or home
    msg -n "Downloading filetransport..."
    naocp "${BASEDIR}" \
        "nao@${NAO}:naoqi/filetransport_*" \
        "${LOGDIR}/${NAO}"
    RETURN_CODE=$?
    naocp "${BASEDIR}" \
        "nao@${NAO}:/mnt/usb/filetransport_*" \
        "${LOGDIR}/${NAO}"
    if [ "$?" -ne "0" ] && [ "$RETURN_CODE" -ne "0" ]; then
      msg -e "Failed to download fileTransport from ${NAO}!"
      ERROR=1
    else
      msg -n "[done]"
    fi

    # download replay from usb or home
    msg -n "Downloading replay..."
    naocp "${BASEDIR}" \
        "nao@${NAO}:naoqi/replay_*" \
        "${LOGDIR}/${NAO}"
    RETURN_CODE=$?
    naocp "${BASEDIR}" \
        "nao@${NAO}:/mnt/usb/replay_*" \
        "${LOGDIR}/${NAO}"
    if [ "$?" -ne "0" ] && [ "$RETURN_CODE" -ne "0" ]; then
      msg -e "Failed to download replay from ${NAO}!"
      ERROR=1
    else
      msg -n "[done]"
    fi

    # download dmesg
    msg -n "Downloading dmesg..."
    naocmd "${BASEDIR}" "${NAO}" "dmesg" > "${LOGDIR}/${NAO}/$(date +%Y-%m-%d_%H-%M-%S)_dmesg.log"
    msg -n "[done]"

    return $ERROR
}
