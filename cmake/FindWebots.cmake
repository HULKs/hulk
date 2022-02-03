find_path(
  Webots_HOME
    "include/controller/cpp/webots/Robot.hpp"
  HINTS
    "/opt/webots"
    "/usr/local/webots"
    "/Applications/Webots"
    "${PATH}"
    "$ENV{HOME}/webots"
    "$ENV{WEBOTS_HOME}"
)

mark_as_advanced(Webots_FOUND Webots_INCLUDE_DIRS Webots_LIBRARIES)

list(APPEND Webots_INCLUDE_DIRS
  "${Webots_HOME}/include/controller/cpp"
  "${Webots_HOME}/include/controller/c"
)

list(APPEND Webots_LIBRARIES
  "${Webots_HOME}/lib/controller/libCppController.so"
  "${Webots_HOME}/lib/controller/libController.so"
)

include(FindPackageHandleStandardArgs)
find_package_handle_standard_args(Webots DEFAULT_MSG Webots_HOME Webots_INCLUDE_DIRS Webots_LIBRARIES)

if(Webots_FOUND AND NOT TARGET Webots::Webots)
    add_library(Webots::Webots INTERFACE IMPORTED)
    set_target_properties(Webots::Webots PROPERTIES
        INTERFACE_INCLUDE_DIRECTORIES "${Webots_INCLUDE_DIRS}"
        INTERFACE_LINK_LIBRARIES "${Webots_LIBRARIES}"
    )
endif()
