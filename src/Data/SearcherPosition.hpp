#pragma once

#include "Framework/DataType.hpp"
#include "Messages/RoboCupGameControlData.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"
#include <vector>


struct SearcherPosition : public DataType<SearcherPosition>
{
public:
  /**
   * @brief SearcherPosition initializes members
   */
  SearcherPosition()
  {
    suggestedSearchPositions.fill(Vector2f::Zero());
    suggestedSearchPositionValid.fill(false);
  };

  /// the name of this DataType
  DataTypeName name__{"SearcherPosition"};
  /// the pose to move the robot to.
  Pose pose;
  /// whether the robot has valid search data
  bool valid = false;
  /// the position to look at to find the ball
  Vector2f searchPosition = Vector2f::Zero();
  /// the positions to look at to find the ball for all other players.
  std::array<Vector2f, MAX_NUM_PLAYERS> suggestedSearchPositions;
  /// If pose and searchPosition is valid.
  bool ownSearchPoseValid = false;
  /// If the suggestedSearchPosition is valid
  /// (per player; maps to the suggestedSearchPositions array).
  std::array<bool, MAX_NUM_PLAYERS> suggestedSearchPositionValid{};
  /// The robot with the oldest, continously updated map (calculated on this robot)
  unsigned int localMostWisePlayerNumber = 0;
  /// The robot with the oldest, continuously updated map (which we got from the robot with the
  /// smallest player number)
  unsigned int globalMostWisePlayerNumber = 0;

  void reset() override
  {
    for (auto& suggestedSearchPosition : suggestedSearchPositions)
    {
      suggestedSearchPosition.setZero();
    }
    for (auto& valid : suggestedSearchPositionValid)
    {
      valid = false;
    }
    ownSearchPoseValid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["pose"] << pose;
    value["searchPosition"] << searchPosition;
    value["suggestedSearchPositions"] << suggestedSearchPositions;
    value["ownSearchPoseValid"] << ownSearchPoseValid;
    value["suggestedSearchPositionsValid"] << suggestedSearchPositionValid;
    value["localMostWisePlayerNumber"] << localMostWisePlayerNumber;
    value["globalMostWisePlayerNumber"] << globalMostWisePlayerNumber;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["pose"] >> pose;
    value["searchPosition"] >> searchPosition;
    value["suggestedSearchPositions"] >> suggestedSearchPositions;
    value["ownSearchPoseValid"] >> ownSearchPoseValid;
    value["suggestedSearchPositionsValid"] >> suggestedSearchPositionValid;
    value["localMostWisePlayerNumber"] >> localMostWisePlayerNumber;
    value["globalMostWisePlayerNumber"] >> globalMostWisePlayerNumber;
  }
};
