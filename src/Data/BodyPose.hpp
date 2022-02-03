#pragma once

#include "Data/ActionCommand.hpp"
#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"


class BodyPose : public DataType<BodyPose>
{
public:
  enum class FallDirection
  {
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
  /// the name of this DataType
  DataTypeName name__{"BodyPose"};
  /// whether the robot is approximately upright
  bool upright;
  /// whether the robot is fallen
  bool fallen;
  /// whether the robot is wonky
  bool wonky;
  /// the time at which the robot started to fall down
  Clock::time_point timeWhenFallen;
  /// the direction in which the robot is falling
  FallDirection fallDirection;
  /// whether at least one foot has contact to something (i.e. the ground)
  bool footContact;
  /// true if the support foot changed within the last cycle
  bool supportChanged;
  /// indicating which of the feet is the support foot (postive if left support)
  float supportSide;
  /// the time at which the robot last had contact with its feet
  Clock::time_point timeOfLastFootContact;
  /**
   * @brief reset sets the state to some defaults
   */
  void reset() override
  {
    upright = false;
    fallen = false;
    wonky = false;
    fallDirection = FallDirection::NOT_FALLING;
    footContact = true;
    supportChanged = false;
    supportSide = 0.f;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["upright"] << upright;
    value["fallen"] << fallen;
    value["wonky"] << wonky;
    value["timeWhenFallen"] << timeWhenFallen;
    value["fallDirection"] << static_cast<int>(fallDirection);
    value["footContact"] << footContact;
    value["supportChanged"] << supportChanged;
    value["supportSide"] << supportSide;
    value["timeOfLastFootContact"] << timeOfLastFootContact;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["upright"] >> upright;
    value["fallen"] >> fallen;
    value["wonky"] >> wonky;
    value["timeWhenFallen"] >> timeWhenFallen;
    int readNumber;
    value["fallDirection"] >> readNumber;
    fallDirection = static_cast<FallDirection>(readNumber);
    value["footContact"] >> footContact;
    value["supportChanged"] >> supportChanged;
    value["supportSide"] >> supportSide;
    value["timeOfLastFootContact"] >> timeOfLastFootContact;
  }
};
