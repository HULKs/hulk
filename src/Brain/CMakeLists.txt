target_sources(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/Brain.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Brain.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/BrainThread.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/BrainThread.hpp"
)

add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Behavior/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/CollisionDetection/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Dribble/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/FootCollisionDetector/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/GameController/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/HeadDamageProvider/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Interface/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Knowledge/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Network/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/OdometryOffsetProvider/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/PathPlanning/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/ReplayRecorder/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/StiffnessLossDetector/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/TeamPlayersAugmenter/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/WhistleDetection/")
