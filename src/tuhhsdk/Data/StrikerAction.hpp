#pragma once

#include "Framework/DataType.hpp"
#include "Tools/BallUtils.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"


class StrikerAction : public DataType<StrikerAction>
{
public:
  /// the name of this DataType
  DataTypeName name = "StrikerAction";
  /**
   * @enum Type enumerates the possible types of action for a striker
   */
  enum class Type
  {
    /// search for the ball
    SEARCH,
    /// kick the ball
    KICK,
    /// pass the ball to a teammate
    PASS,
    /// dribble the ball to a position
    DRIBBLE,
    /// walk to pose
    WALK,
    /// InWalkKick
    IN_WALK_KICK,
    /// no action specified
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
  /// type of in walk kick we want to do
  InWalkKickType inWalkKickType;
  /// the relative pose from where we want to kick from
  Pose kickPose;
  /// if ball is kickable at the moment and how
  BallUtils::Kickable kickable;

  void reset() override
  {
    valid = false;
    type = Type::NONE;
    passTarget = 0;
    target = Vector2f::Zero();
    kickType = KickType::NONE;
    inWalkKickType = InWalkKickType::NONE;
    kickPose = Pose();
    kickable = BallUtils::Kickable::NOT;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["valid"] << valid;
    value["type"] << static_cast<int>(type);
    value["passTarget"] << passTarget;
    value["target"] << target;
    value["kickType"] << static_cast<int>(kickType);
    value["inWalkKickType"] << static_cast<int>(inWalkKickType);
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
    value["inWalkKickType"] >> readNumber;
    inWalkKickType = static_cast<InWalkKickType>(readNumber);
    value["kickPose"] >> kickPose;
    value["kickable"] >> readNumber;
    kickable = static_cast<BallUtils::Kickable>(readNumber);
  }
};
