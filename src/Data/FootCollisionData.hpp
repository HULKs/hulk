#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"

class FootCollisionData : public DataType<FootCollisionData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"FootCollisionData"};
  // Whether the left or right foot bumper detected a collision
  bool collision = false;
  /// the timestamps at which the foot collisions were detected
  Clock::time_point timestamp;
  // Whether the data is valid
  bool valid = false;
  /**
   * @brief reset clears the sequence of samples
   */
  void reset() override
  {
    collision = false;
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["collision"] << collision;
    value["timestamp"] << timestamp;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["collision"] >> collision;
    value["timestamp"] >> timestamp;
    value["valid"] >> valid;
  }
};
