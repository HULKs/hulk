#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"


struct BallSearchPosition : public DataType<BallSearchPosition>
{
public:
  /// the pose to move the robot to.
  Pose pose;
  /// the position to look at to find the ball
  Vector2f searchPosition;
  /// the positions to look at to find the ball for all other players.
  VecVector2f suggestedSearchPositions;

  void reset()
  {
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["pose"] << pose;
    value["searchPosition"] << searchPosition;
    value["suggestedSearchPositions"] << suggestedSearchPositions;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["pose"] >> pose;
    value["searchPosition"] >> searchPosition;
    value["suggestedSearchPositions"] >> suggestedSearchPositions;
  }
};
