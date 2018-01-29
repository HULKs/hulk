#pragma once

#include "Framework/DataType.hpp"
#include "Tools/BallUtils.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Math/Eigen.hpp"


class StrikerAction : public DataType<StrikerAction>
{
public:
  /**
   * @enum Type enumerates the possible types of action for a striker
   */
  enum Type
  {
    /// kick the ball into the goal
    KICK_INTO_GOAL,
    /// dribble the ball into the goal
    DRIBBLE_INTO_GOAL,
    /// pass the ball to a teammate
    PASS,
    /// dribble the ball to a position
    DRIBBLE,
    /// wait for keeper playing the ball
    WAITING_FOR_KEEPER
  };
  enum KickType
  {
    /// the plain old schlong kick
    CLASSIC,
    /// the fancy new DMP-Kick
    STRAIGHT,
    /// the gentle in-walk
    IN_WALK_GENTLE,
    /// the strong in-walk
    IN_WALK_STRONG
  };
  /// true iff this struct is valid
  bool valid;
  /// the type of the action
  Type type;
  /// the player number of the pass target (for pass action)
  unsigned int passTarget;
  /// the field coordinates of the ball target
  Vector2f target;
  /// type of kick we want to do
  KickType kickType;
  /// the relative pose from where we want to kick from
  Pose kickPose;
  /// if ball is kickable at the moment and how
  BallUtils::Kickable kickable;
  /**
   * @brief reset does nothing
   */
  void reset()
  {
    valid = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["valid"] << valid;
    value["type"] << static_cast<int>(type);
    value["passTarget"] << passTarget;
    value["target"] << target;
    value["kickType"] << static_cast<int>(kickType);
    value["kickPose"] << kickPose;
    value["kickable"] << static_cast<int>(kickable);
  }
  virtual void fromValue(const Uni::Value& value)
  {
    value["valid"] >> valid;
    int readNumber = 0;
    value["type"] >> readNumber;
    type = static_cast<Type>(readNumber);
    value["passTarget"] >> passTarget;
    value["target"] >> target;
    value["kickType"] >> readNumber;
    kickType = static_cast<KickType>(readNumber);
    value["kickPose"] >> kickPose;
    value["kickable"] >> readNumber;
    kickable = static_cast<BallUtils::Kickable>(readNumber);
  }
};
