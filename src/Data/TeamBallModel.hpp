#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Math/Eigen.hpp"


class TeamBallModel : public DataType<TeamBallModel>
{
public:
  /// the name of this DataType
  DataTypeName name__{"TeamBallModel"};
  enum class BallType
  {
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
  BallType ballType = BallType::NONE;
  /// true if ball is inside field
  bool insideField = false;
  /// indicates whether a team member saw the ball confidently enough
  bool seen = false;
  /// indicates whether a consensus of multiple balls could be made
  bool found = false;
  /// the position of the common ball in field coordinates
  Vector2f absPosition = Vector2f::Zero();
  /// the position of the common ball in robot coordinates
  Vector2f relPosition = Vector2f::Zero();
  /// the velocity of the common ball [m/s]
  Vector2f velocity = Vector2f::Zero();
  /// the last time point the ball was seen
  Clock::time_point timeLastUpdated;
  /**
   * @brief reset resets the found state
   */
  void reset() override
  {
    ballType = BallType::NONE;
    seen = false;
    found = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["ballType"] << static_cast<int>(ballType);
    value["insideField"] << insideField;
    value["seen"] << seen;
    value["found"] << found;
    value["absPosition"] << absPosition;
    value["relPosition"] << relPosition;
    value["velocity"] << velocity;
    value["timeLastUpdated"] << timeLastUpdated;
  }

  void fromValue(const Uni::Value& value) override
  {
    int input = 0;
    value["ballType"] >> input;
    ballType = static_cast<BallType>(input);
    value["insideField"] >> insideField;
    value["seen"] >> seen;
    value["found"] >> found;
    value["absPosition"] >> absPosition;
    value["relPosition"] >> relPosition;
    value["velocity"] >> velocity;
    value["timeLastUpdated"] >> timeLastUpdated;
  }
};
