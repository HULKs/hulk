#pragma once

#include "Data/BallState.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/StrikerAction.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Framework/Module.hpp"


class Brain;

class StrikerActionProvider : public Module<StrikerActionProvider, Brain>
{
public:
  /**
   * @brief StrikerActionProvider initializes members
   * @param manager a reference to brain
   */
  StrikerActionProvider(const ModuleManagerInterface& manager);
  /**
   * @brief cycle updates the striker action
   */
  void cycle();

private:
  struct PassTarget
  {
    unsigned int number;
    float rating;
    Vector2f position;
  };

  static constexpr float lastTargetBonus_ = 1.f;

  const Parameter<bool> checkIfKeeperWantsToPlayBall_;
  /// whether the robot should try to shoot into the goal or not (it will dribble then)
  const Parameter<bool> shootIntoGoal_;
  const Parameter<float> distanceToBallDribble_;
  Parameter<float> angleToBallDribble_;
  const Parameter<float> distanceToBallKick_;
  Parameter<float> angleToBallKick_;
  /**
   * when this parameter is !=0, the kickPose will always lead to a pose to kick with the left (1) or right (-1) foot.
   */
  const Parameter<int> useOnlyThisFoot_;

  const Dependency<BallState> ballState_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamPlayers> teamPlayers_;
  const Dependency<GameControllerState> gameControllerState_;


  StrikerAction::Type lastAction_;
  int lastSign_;
  unsigned int lastPassTarget_;
  float penaltyTargetOffset_;

  /// a reference to the striker action
  Production<StrikerAction> strikerAction_;
  bool keeperWantsToPlayBall() const;
  void calculateStrikerAction();
  void calculateKick();
  void calculatePenaltyStrikerAction();

  float ratePosition(const Vector2f& position) const;

  StrikerActionProvider::PassTarget findPassTarget(const float ballRating) const;
};
