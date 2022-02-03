target_sources(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/AudioInterface.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Clock.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Definitions.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Definitions.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/FakeDataInterface.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/JointUtils.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/RobotInterface.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/RobotMetrics.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/RobotMetrics.hpp"
)

add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Kinematics/")
if("${HULK_TARGET}" STREQUAL "NAO")
  add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Nao/")
elseif("${HULK_TARGET}" STREQUAL "Replay")
  add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Replay/")
elseif("${HULK_TARGET}" STREQUAL "SimRobot")
  add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/SimRobot/")
elseif("${HULK_TARGET}" STREQUAL "Webots")
  add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Webots/")
endif()
