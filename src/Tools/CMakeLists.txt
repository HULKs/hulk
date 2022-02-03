target_sources(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/BallUtils.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Chronometer.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Chronometer.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/FieldDimensionUtils.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/PermissionManagement.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/SelectWalkMode.hpp"
)

add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Backtrace/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Math/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/PathPlanning/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/StateEstimation/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/StateMachine/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Storage/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Var/")
