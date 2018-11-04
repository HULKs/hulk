#pragma once

#include "Framework/DataType.hpp"
#include "Tools/BallUtils.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Math/Eigen.hpp"


class StrikerAction : public DataType<StrikerAction>
{
public:
  /// the name of this DataType
  DataTypeName name = "StrikerAction";
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
    /// forward kick
    FORWARD,
    /// side kick
    SIDE,
    /// the gentle in-walk
    IN_WALK_GENTLE,
    /// the strong in-walk
    IN_WALK_STRONG,
    /// don't kick
    NONE
  };
  /// true iff this struct is valid
  bool valid = false;
  /// the type of the action
  Type type = Type::DRIBBLE;
  /// the player number of the pass target (for pass action)
  unsigned int passTarget = 0;
  /// the field coordinates of the ball target
  Vector2f target = Vector2f::Zero();
  /// type of kick we want to do
  KickType kickType;
  /// the relative pose from where we want to kick from
  Pose kickPose;
  /// if ball is kickable at the moment and how
  BallUtils::Kickable kickable;
  /**
   * @brief reset does nothing
   */
  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
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
  void fromValue(const Uni::Value& value) override
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
