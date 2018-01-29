#pragma once

#include "Tools/Time.hpp"
#include "Framework/DataType.hpp"

#include "Tools/Math/Eigen.hpp"

class BallState : public DataType<BallState> {
public:
  /// position (meters) of the ball relative to the robot
  Vector2f position;
  /// velocity (meters per second) of the ball relative to the robot
  Vector2f velocity;
  /// the predicted ball destination
  Vector2f destination;
  /// time (seconds) since the last valid ball data arrived
  float age;
  /// true iff the ball is found
  bool found;
  /// true iff the ball moved significantly during the last cycle
  bool moved;
  /// true iff the filter is really confident that it is the correct ball
  bool confident;
  /// head yaw angle (radians) that would be necessary to have the ball in the center of the camera image
  float headYaw;
  /// the time when the ball was lost
  TimePoint timeWhenBallLost;
  /// the time when the ball was seen
  TimePoint timeWhenLastSeen;
  /**
   * @brief reset invalidates the data
   */
  void reset()
  {
    moved = false;
    found = false;
    confident = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["position"] << position;
    value["velocity"] << velocity;
    value["destination"] << destination;
    value["age"] << age;
    value["found"] << found;
    value["moved"] << moved;
    value["confident"] << confident;
    value["headYaw"] << headYaw;
    value["timeWhenBallLost"] << timeWhenBallLost;
    value["timeWhenLastSeen"] << timeWhenLastSeen;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["position"] >> position;
    value["velocity"] >> velocity;
    value["destination"] >> destination;
    value["age"] >> age;
    value["found"] >> found;
    value["moved"] >> moved;
    value["confident"] >> confident;
    value["headYaw"] >> headYaw;
    value["timeWhenBallLost"] >> timeWhenBallLost;
    value["timeWhenLastSeen"] >> timeWhenLastSeen;
  }
};
