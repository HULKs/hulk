#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class TeamBallModel : public DataType<TeamBallModel> {
public:
  enum class BallType {
    /// no ball at all
    NONE,
    /// ball from BallModel
    SELF,
    /// ball from TeamBallModel
    TEAM,
    /// ball position known due to rules (in READY and SET)
    RULE
  };
  /// the type of the ball as enumerated above
  BallType ballType;
  /// true if ball is inside field
  bool insideField;
  /// indicates whether a team member saw the ball confidently enough
  bool seen;
  /// indicates whether a consensus of multiple balls could be made
  bool found;
  /// the position of the common ball in field coordinates
  Vector2f position;
  /// the velocity of the common ball in field coordinates
  Vector2f velocity;
  /**
   * @brief reset resets the found state
   */
  void reset()
  {
    ballType = BallType::NONE;
    seen = false;
    found = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["ballType"] << static_cast<int>(ballType);
    value["insideField"] << insideField;
    value["seen"] << seen;
    value["found"] << found;
    value["position"] << position;
    value["velocity"] << velocity;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    int input = 0;
    value["ballType"] >> input;
    ballType = static_cast<BallType>(input);
    value["insideField"] >> insideField;
    value["seen"] >> seen;
    value["found"] >> found;
    value["position"] >> position;
    value["velocity"] >> velocity;
  }
};
