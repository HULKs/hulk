# Find ittnotify
#
# This module defines
# ITTNOTIFY_INCLUDE_DIRS
# ITTNOTIFY_LIBRARIES
# ITTNOTIFY_FOUND

if (CMAKE_SIZEOF_VOID_P MATCHES "8")
  set(vtune_lib_dir lib64)
else()
  set(vtune_lib_dir lib32)
endif()

find_path(ITTNOTIFY_INCLUDE_DIR NAMES ittnotify.h HINTS $ENV{ITTNOTIFY_HOME}/include $ENV{VTUNE_HOME}/include ${VTUNE_HOME}/include NO_CMAKE_FIND_ROOT_PATH)
find_library(ITTNOTIFY_LIBRARY NAMES ittnotify HINTS $ENV{ITTNOTIFY_HOME}/${vtune_lib_dir} $ENV{VTUNE_HOME}/${vtune_lib_dir} ${VTUNE_HOME}/${vtune_lib_dir} NO_CMAKE_FIND_ROOT_PATH)

set(ITTNOTIFY_INCLUDE_DIRS ${ITTNOTIFY_INCLUDE_DIR})
set(ITTNOTIFY_LIBRARIES ${ITTNOTIFY_LIBRARY})

include(FindPackageHandleStandardArgs)
find_package_handle_standard_args(ITTNotify DEFAULT_MSG ITTNOTIFY_LIBRARY ITTNOTIFY_INCLUDE_DIR)

mark_as_advanced(ITTNOTIFY_INCLUDE_DIR ITTNOTIFY_LIBRARY)
