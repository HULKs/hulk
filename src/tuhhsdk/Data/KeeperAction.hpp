#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Math/Eigen.hpp"

class KeeperAction : public DataType<KeeperAction>
{
public:
  /**
   * @enum Type enumerates the possible types of action for a keeper
   */
  enum Type
  {
    /// go to default position
    GO_TO_DEFAULT_POS,
    /// search for ball
    SEARCH_FOR_BALL,
    /// Kick the ball asap away
    KICK_BALL_ASAP_AWAY,
    /// Go closer to Ball
    GO_CLOSER_TO_CLOSE_BALL,
    /// Do genuflect (sit down and spread legs)
    GENUFLECT
  };

  /// the field coordinates of the ball target
  Vector2f target;
  /// the type of the action
  Type type;
  /// true iff this struct is valid
  bool valid;
  /// the position walk to
  Pose walkPosition;
  /// indicate if Keeper wants to play ball
  bool wantsToPlayBall = false;
  /**
   * @brief reset does nothing
   */
  void reset()
  {
    valid = false;
    wantsToPlayBall = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["target"] << target;
    value["type"] << static_cast<int>(type);
    value["valid"] << valid;
    value["walkPosition"] << walkPosition;
    value["wantsToPlayBall"] << wantsToPlayBall;
  }
  virtual void fromValue(const Uni::Value& value)
  {
    value["target"] >> target;
    int readNumber = 0;
    value["type"] >> readNumber;
    type = static_cast<Type>(readNumber);
    value["valid"] >> valid;
    value["walkPosition"] >> walkPosition;
    value["wantsToPlayBall"] >> wantsToPlayBall;
  }
};
