#pragma once

#include "Data/BallState.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/KickConfigurationData.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/SetPlayStrikerAction.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"

class Brain;

class SetPlayStrikerActionProvider : public Module<SetPlayStrikerActionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "SetPlayStrikerActionProvider";
  /**
   *@brief The constructor of this class
   */
  explicit SetPlayStrikerActionProvider(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  /**
   * @brief set all relevant members of the striker action for walking to a specific pose
   * @param walkTarget a Pose where the robot shall walk to
   */
  void createStrikerAction(const Pose& walkTarget);
  /**
   * @brief set all relevant members of the striker action for dribbling
   * @param absTarget where the ball should be dribbled to
   * @param relBallPosition the relative ball position
   * @param lastSign the sign of the foot that should dribble in the last decision (1 for left,
   * -1 for right)
   * @param forceSign whether lastSign must not be changed (can be used to enforce dribbling
   * with a specific foot)
   */
  void createStrikerAction(const Vector2f& absTarget, const Vector2f& relBallPosition,
                           int& lastSign, const bool forceSign);
  /**
   * @brief set all relevant members of the striker action for a kick
   * @param kickType the type of the kick
   * @param absTarget where the ball should be kicked to
   * @param relBallPosition the relative ball position
   * @param lastSign the sign of the foot that should kick in the last decision (1 for left, -1 for
   * right)
   * @param forceSign whether lastSign must not be changed (can be used to enforce kicking with a
   * specific foot)
   */
  void createStrikerAction(const KickType kickType, const Vector2f& absTarget,
                           const Vector2f& relBallPosition, int& lastSign, const bool forceSign);
  /**
   * @brief block line of sight between ball and own goal (during defensive set play)
   */
  void block();
  /**
   * @brief kick or dribble (during offensive set play)
   *
   * This method handles a finite state machine for non-none set plays where we are the kicking
   * team. Depending on the situation on the field (ball position, type of set play, opposing and
   * allied robots) the ball is either kicked or dribbled.
   * @see kickOrDribble
   */
  void performFreeKick();
  /**
   * @brief decide on a kick (or dribble) target
   *
   * The decision depends on the type of set play and the position of the ball on the field.
   *
   * @return the kick (or dribble) target
   */
  Vector2f kickTarget() const;
  /**
   * @brief decide whether kicking or dribbling is favorable
   *
   * Kicking is return is the ball is close to the opponent's goal or if a pass target exists. It
   * can be disabled by the enableKick_ parameter.
   *
   * @return whether kicking or dribbling is favorable
   */
  SetPlayStrikerAction::Type kickOrDribble();

  const Dependency<BallState> ballState_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<KickConfigurationData> kickConfigurationData_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<TeamPlayers> teamPlayers_;
  const Dependency<WorldState> worldState_;
  Production<SetPlayStrikerAction> setPlayStrikerAction_;
  /// Whether or not the nao is allowed to kick to score a goal during offensive set play
  const Parameter<bool> enableScoring_;
  /// Whether or not the nao is allowed to pass during offensive set play
  const Parameter<bool> enablePassing_;
  const Parameter<Vector2f> distanceToBallDribble_;
  Parameter<float> angleToBallDribble_;
  Parameter<float> angleToBallKick_;
  /// the kick target during a corner kick is in front of the opponent's goal
  const Parameter<float> cornerKickTargetOffset_;
  /// remember kick decision
  bool shouldKick_;
  /// remember foot decision
  int lastSign_;
  /// whether the ball is near the opponent's goal
  bool ballNearOpponentGoal_;
};
