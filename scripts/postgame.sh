#!/bin/bash

# Get base directory for better referencing
BASEDIR=`cd $(dirname $0); pwd -P`
BASEDIR=${BASEDIR%/*}

source "${BASEDIR}/scripts/lib/msg.sh"
source "${BASEDIR}/scripts/lib/naonet.sh"
source "${BASEDIR}/scripts/lib/numberToIP.sh"
source "${BASEDIR}/scripts/lib/docker.sh"
source "${BASEDIR}/scripts/lib/logs.sh"

function helpMenu {
  echo "Usage: $0 [OPTIONS] NAO..."
  echo ""
  echo "Options"
  echo " -l, --logdir LOGDIR               the directory to which the logs from the NAO are downloaded"
  echo " -n, --network NETWORK             the network to which the robots should be set (e.g. ETH or SPL_A)"
  echo " -h, --help                        show this help"
  echo ""
  echo "Nao"
  echo " either hostname, IP or number of the NAO"
}

function printErrorMessage {
  case $1 in
    0)
      msg -s "Finised postgame for $2"
      ;;
    1)
      msg -e "Failed to stop the hulk service on $2!"
      ;;
    2)
      msg -e "No logs were downloaded from $2 because the hulk service could not be stopped!"
      ;;
    3)
      msg -e "Failed to download logs in home folder from $2!"
      ;;
    4)
      msg -e "Failed to download logs on usb stick from $2!"
      ;;
  esac
}

function run {
  # a list of NAOs that are prepared
  NAOS=()
  ERRORS=()
  # default network is ethernet
  NETWORK=ETH
  # empty logdir means that no logs are downloaded
  LOGDIR=""
  # prepare parameters
  PARAMETERS=""
  while [ "$#" -gt 0 ]; do
    if [[ $1 =~ ^- ]] && [[ ! $1 =~ ^-- ]]; then
      PARAMETERS+=$(echo "${1:1}" | sed -e "s/\(.\)/ -\1/g")
    else
      PARAMETERS+=" $1"
    fi
    shift
  done
  eval set -- "${PARAMETERS}";

  while [[ "$1" =~ ^- ]]; do
    case "$1" in
      "-l" | "--logdir")
        shift
        if [ "$#" -eq 0 ]; then
          msg -e "--logdir needs a directory as parameter!"
          helpMenu
          return 1
        fi
        # If you really need to use the --logdir option, you can comment the following 4 lines.
        # You have to use a name that is valid inside the container, though, such as
        # --logdir /nao/logs/Game3
        if iAmInDocker; then
          msg -e "Sorry, but the --logdir option is not yet available in docker!"
          return 1
        fi
        LOGDIR="$1"
        ;;
      "-n" | "--network")
        shift
        if [ "$#" -eq 0 ]; then
          msg -e "--network needs a network as parameter!"
          helpMenu
          return 1
        fi
        NETWORK="$1"
        ;;
      "-h" | "--help")
        helpMenu
        return 0
        ;;
      *)
        msg -e "Failed to parse \"$1\"!"
        helpMenu
        return 1
        ;;
    esac
    shift
  done

  if [ "$#" -lt 1 ]; then
    helpMenu
    return 1
  fi

  NAOS=()
  NAONUMBERS=()
  while [ "$#" -gt 0 ]; do
    NAOS+=($(numberToIP "$1"))
    NAONUMBERS+=("$1")
    shift
  done

  ERROR=0
  for index in ${!NAOS[*]}; do
    ERRORS[index]=0
    msg -n "Stopping ${NAOS[index]}."
    if [[ "${NAONUMBERS[index]}" -gt 19 ]]; then
      naocmd "${BASEDIR}" "${NAOS[index]}" "systemctl --user stop hulk.service; /data/home/nao/.local/bin/setNetwork ${NETWORK}; exit 0"
    else
      naocmd "${BASEDIR}" "${NAOS[index]}" "sudo /etc/init.d/hulk stop; /home/nao/bin/setNetwork ${NETWORK}; exit 0"
    fi
    if [ "$?" -ne 0 ]; then
      ERRORS[index]=1
      printErrorMessage 1 ${NAOS[index]}
      if [ ! "${LOGDIR}" == "" ]; then
        ERRORS[index]=2
        printErrorMessage 2 ${NAOS[index]}
      fi
      ERROR=1
      continue
    fi

    # download logs
    if [ ! "${LOGDIR}" == "" ]; then
      download_logs "$BASEDIR" "${NAOS[index]}" "$LOGDIR"
      delete_logs "$BASEDIR" "${NAOS[index]}"
    fi
    msg -s "Finished with ${NAOS[index]}!"
  done

  echo "---------------------"
  echo "Summary for postgame:"
 
  for index in ${!ERRORS[*]}
  do
    printErrorMessage ${ERRORS[index]} ${NAOS[index]}
  done


  return ${ERROR}
}

handleDocker "${BASEDIR}" "$@"
