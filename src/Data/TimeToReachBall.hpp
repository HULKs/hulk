#pragma once

#include "Data/TeamPlayers.hpp"
#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"


class TimeToReachBall : public DataType<TimeToReachBall>
{
public:
  /// the name of this DataType
  DataTypeName name__{"TimeToReachBall"};
  /// the estimated time the robot needs to reach the ball
  Clock::duration timeToReachBall = 1337s;
  /// the estimated time the robot needs to reach the ball as striker
  Clock::duration timeToReachBallStriker = 1337s;
  /// the function to calculate the estimated time to reach a ball
  std::function<Clock::duration(Pose player, Vector2f ballPosition, Vector2f target, bool fallen,
                                bool ballSeen, Pose maxVelocityComponents)>
      estimateTimeToReachBall;
  /// whether the time to reach ball is valid
  bool valid = false;
  /**
   * @brief reset invalidates the data
   */
  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["timeToReachBall"] << timeToReachBall;
    value["timeToReachBallStriker"] << timeToReachBallStriker;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["timeToReachBall"] >> timeToReachBall;
    value["timeToReachBallStriker"] >> timeToReachBallStriker;
    value["valid"] >> valid;
  }
};
