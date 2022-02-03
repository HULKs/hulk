#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Pose.hpp"


class LoserPosition : public DataType<LoserPosition>
{
public:
  /// the name of this DataType
  DataTypeName name__{"LoserPosition"};
  /// whether the loser position is valid
  bool valid = false;
  /// the pose of the loser (in relative coordinates)
  Pose pose;
  /**
   * @brief invalidates the position
   */
  void reset() override
  {
    valid = false;
    pose = Pose{};
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
