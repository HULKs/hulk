#!/bin/bash

DATE_CMD="date"
if [[ `uname` == "Darwin" ]]; then
  DATE_CMD="gdate"
fi

# Set start point for time measurement
function set_start_time {
  START_TIME="$(($(${DATE_CMD} +%s%N)))"
}

# Get time difference
function get_time_diff {
  S="$(($(($(${DATE_CMD} +%s%N)-${START_TIME}))/1000000000))"
  M="$(($(($(${DATE_CMD} +%s%N)-${START_TIME}))/1000000))"
  printf -v DIFF "%02d:%02d:%02d.%03d" "$((S/3600%24))" "$((S/60%60))" "$((S%60))" "$((M%1000))"
}
