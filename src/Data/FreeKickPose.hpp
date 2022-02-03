#pragma once

#include "Framework/DataType.hpp"

#include "Tools/Math/Pose.hpp"


class FreeKickPose : public DataType<FreeKickPose>
{
public:
  /// the name of this DataType
  DataTypeName name__{"FreeKickPose"};
  /// whether the free kick pose is valid
  bool valid = false;
  /// the pose where the robot should be if the game controller is in the free kick state.
  Pose pose;

  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["valid"] << valid;
    value["pose"] << pose;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["valid"] >> valid;
    value["pose"] >> pose;
  }
};
