#pragma once

#include <Framework/DataType.hpp>
#include <Tools/Time.hpp>

class FootCollisionData : public DataType<FootCollisionData>
{
public:
  /// the name of this DataType
  DataTypeName name = "FootCollisionData";
  // Whether the left or right foot bumper detected a collision
  bool collision = false;
  /// the timestamps at which the foot collisions were detected
  TimePoint timestamp;
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
