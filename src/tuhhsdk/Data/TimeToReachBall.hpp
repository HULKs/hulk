#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Time.hpp"


class TimeToReachBall : public DataType<TimeToReachBall>
{
public:
  /// the name of this DataType
  DataTypeName name = "TimeToReachBall";
  /// the estimated time (seconds) the robot needs to reach the ball
  float timeToReachBall = 1337.f;
  /// the estimated time (seconds) the robot needs to reach the ball as striker
  float timeToReachBallStriker = 1337.f;
  /// whether the time to reach ball is valid
  bool valid = false;
  /**
   * @brief reset invalidates the data
   */
  void reset()
  {
    valid = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["timeToReachBall"] << timeToReachBall;
    value["timeToReachBallStriker"] << timeToReachBallStriker;
    value["valid"] << valid;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["timeToReachBall"] >> timeToReachBall;
    value["timeToReachBallStriker"] >> timeToReachBallStriker;
    value["valid"] >> valid;
  }
};
