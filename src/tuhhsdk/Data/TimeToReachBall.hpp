#pragma once

#include "Data/TeamPlayers.hpp"
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
  /// the function to calculate the estimated time to reach a ball
  std::function<float(Pose player, Vector2f ballPosition, Vector2f target, bool fallen,
                      bool ballSeen, Pose maxVelocityComponents, float walkAroundBallVelocity)>
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
