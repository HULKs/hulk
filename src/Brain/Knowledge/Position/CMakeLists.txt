target_sources(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/FakeRobotPoseProvider.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/FakeRobotPoseProvider.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/FieldInfo.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/FieldInfo.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/PoseHypothesis.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/PoseHypothesis.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/PositionProvider.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/PositionProvider.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/UKFPose2D.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/UKFPose2D.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/UKFPositionKnowledge.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/UKFPositionKnowledge.hpp"
)
