#pragma once

#include <Tools/Time.hpp>
#include <Framework/DataType.hpp>

enum class FallDirection {
  /// the robot is not falling
  NOT_FALLING,
  /// the robot is falling forwards
  FRONT,
  /// the robot is falling backwards
  BACK,
  /// the robot is falling to the right
  RIGHT,
  /// the robot is falling to the left
  LEFT
};

class BodyPose : public DataType<BodyPose> {
public:
  /// whether the robot is fallen
  bool fallen;
  /// the time at which the robot started to fall down
  TimePoint timeWhenFallen;
  /// the direction in which the robot is falling
  FallDirection fallDirection;
  /// whether at least one foot has contact to something (i.e. the ground)
  bool footContact;
  /// the time at which the robot last had contact with its feet
  TimePoint timeOfLastFootContact;
  /**
   * @brief reset sets the state to some defaults
   */
  void reset()
  {
    fallen = false;
    fallDirection = FallDirection::NOT_FALLING;
    footContact = true;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["fallen"] << fallen;
    value["timeWhenFallen"] << timeWhenFallen;
    value["fallDirection"] << static_cast<int>(fallDirection);
    value["footContact"] << footContact;
    value["timeOfLastFootContact"] << timeOfLastFootContact;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["fallen"] >> fallen;
    value["timeWhenFallen"] >> timeWhenFallen;
    int readNumber;
    value["fallDirection"] >> readNumber;
    fallDirection = static_cast<FallDirection>(readNumber);
    value["footContact"] >> footContact;
    value["timeOfLastFootContact"] >> timeOfLastFootContact;
  }
};
