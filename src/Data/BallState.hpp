#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Math/Eigen.hpp"

class BallState : public DataType<BallState>
{
public:
  /// the name of this DataType
  DataTypeName name__{"BallState"};
  /// position (meters) of the ball relative to the robot
  Vector2f position{Vector2f::Zero()};
  /// velocity (meters per second) of the ball relative to the robot
  Vector2f velocity{Vector2f::Zero()};
  /// the predicted ball destination
  Vector2f destination{Vector2f::Zero()};
  /// time since the last valid ball data arrived
  Clock::duration age{1337s};
  /// true iff a ball was found at all
  bool found{false};
  /// true iff the ball is assumed to be moving
  bool moved{false};
  /// true iff the filter has seen this ball a couple of times
  bool confident{false};
  /// the time when the ball was lost
  Clock::time_point timeWhenBallLost;
  /// the time when the ball was seen
  Clock::time_point timeWhenLastSeen;
  /// a scalar that represents the validity of this ball (higher is better)
  float validity;

  /**
   * @brief reset invalidates the data
   */
  void reset() override
  {
    moved = false;
    found = false;
    confident = false;
    validity = 0.f;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["position"] << position;
    value["velocity"] << velocity;
    value["destination"] << destination;
    value["age"] << age;
    value["found"] << found;
    value["moved"] << moved;
    value["confident"] << confident;
    value["timeWhenBallLost"] << timeWhenBallLost;
    value["timeWhenLastSeen"] << timeWhenLastSeen;
    value["validity"] << validity;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["position"] >> position;
    value["velocity"] >> velocity;
    value["destination"] >> destination;
    value["age"] >> age;
    value["found"] >> found;
    value["moved"] >> moved;
    value["confident"] >> confident;
    value["timeWhenBallLost"] >> timeWhenBallLost;
    value["timeWhenLastSeen"] >> timeWhenLastSeen;
    value["validity"] >> validity;
  }
};
