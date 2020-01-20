#pragma once

#include "Data/BallState.hpp"
#include "Data/CollisionDetectorData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/KickConfigurationData.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/SetPosition.hpp"
#include "Data/StrikerAction.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamObstacleData.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"


class Brain;

class StrikerActionProvider : public Module<StrikerActionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "StrikerActionProvider";
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

  /// angle threshold for kickable orientation check
  Parameter<float> angleToBallDribble_;
  Parameter<float> angleToBallKick_;
  Parameter<float> asapDeviationAngle_;
  /// offset (x, y) of the kickPose for dribble
  const Parameter<Vector2f> distanceToBallDribble_;
  /// direction vectors influencing the dribble direction
  const Parameter<std::vector<std::vector<Vector2f>>> dribbleMapInterpolationPoints_;
  /// kick the ball away from our own goal
  const Parameter<bool> kickAwayFromGoal_;
  /// kick the ball into the opponent goal
  const Parameter<bool> kickIntoGoal_;
  /// the maximum angle the ball will be kicked in reality
  Parameter<float> kickOpeningAngle_;
  /// the weight distribution over the kick opening angle: the possibility to kick the ball to this
  /// weight in reality
  const Parameter<std::vector<float>> kickRatingChunkWeights_;
  /// minimum required probability of the aimed direction to rate the kick to be done (=way is free
  /// enough)
  const Parameter<std::vector<float>> kickRatingThreshold_;
  /// radius from own goal center that the ball is assumed to be near to the own goal
  const Parameter<float> ownGoalAreaRadius_;
  /// radius from opponent goal center that the ball is assumed to be near to the opponent goal
  const Parameter<float> opponentGoalAreaRadius_;
  const Parameter<bool> useInWalkKickAsStrongDribble_;
  const Parameter<bool> useInWalkKickInKickOff_;
  const Parameter<bool> useInWalkKickToClearBall_;
  const Parameter<bool> useInWalkKickToClearBallASAP_;
  const Parameter<bool> useInWalkKickToScoreGoal_;
  /// when this is != 0, the kickPose will always lead to a pose to kick with the left (1) or right
  /// (-1) foot
  const Parameter<int> useOnlyThisFoot_;
  /// whether to use the side kick [NOT IMPLEMENTED]
  const Parameter<bool> useSideKickParam_;
  /// whether to use the strong dribble in the field center
  const Parameter<bool> useStrongDribble_;
  const Parameter<bool> useTurnKickParam_;
  /// always kick no matter what
  const Parameter<bool> forceKick_;

  const Dependency<BallState> ballState_;
  const Dependency<CollisionDetectorData> collisionDetectorData_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<KickConfigurationData> kickConfigurationData_;
  const Dependency<TeamObstacleData> obstacleData_;
  const Dependency<RobotPosition> robotPosition_;
  /// set position needs to be a reference because set position depends on playing roles, which
  /// depends on time to reach ball, which depends on striker action
  const Reference<SetPosition> setPosition_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<TeamPlayers> teamPlayers_;
  const Dependency<WorldState> worldState_;

  /// used for hysteresis in findPassTarget
  StrikerAction::Type lastAction_;
  /// Hysteresis
  bool lastIsBallNearOpponentGoal_;
  bool lastIsBallNearOwnGoal_;
  bool lastKickRating_;
  int lastSign_;
  unsigned int lastPassTarget_;

  /// a reference to the striker action
  Production<StrikerAction> strikerAction_;

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
   * @brief set all relevant members of the striker action for dribbling
   * @param absTarget where the ball should be dribbled to
   * @param relBallPosition the relative ball position
   * @param relBallPosition the relative ball position
   * @param lastSign the sign of the foot that should dribble in the last decision (1 for left, -1
   * for right)
   * @param forceSign whether lastSign must not be changed (can be used to enforce dribbling with a
   * specific foot)
   */
  void createStrikerAction(const Vector2f& absTarget, const Vector2f& relBallPosition,
                           int& lastSign, const bool forceSign);

  /**
   * @brief set all relevant members of the striker action for an in walk kick
   * @param inWalkKickType the type of the in walk kick
   * @param absTarget where the ball should be in-walk-kicked to
   * @param relBallPosition the relative ball position
   */
  void createStrikerAction(const InWalkKickType inWalkKickType, const Vector2f& absTarget,
                           const Vector2f& relBallPosition);

  /**
   * @brief
   * @param
   * @return
   */
  /**
   * @brief Get the current dribble direction at the position of the teamball. Sum all directions at
   * the dribbleMapInterpolationPoints up and weight them with their squared distance.
   * @return the interpolated direction
   */
  Vector2f getInterpolatedDirection() const;
  /**
   * @brief check whether a kick is useful in the current situation. Check for all weights in the
   * kickOpeningAngle if this way is blocked by an obstacle and gauge if enough weights
   * (kickRatingThreshold) are not blocked
   * @param kickTarget absolute target the shoot is aming to hit
   * @param leftClipPoint absolute left limit point for evaluation
   * @param rightClipPoint absolute right limit point for evaluation
   * @return whether a kick is will the reach the target with a given probability
   */
  bool rateKick(const Vector2f& kickTarget, Vector2f leftClipPoint, Vector2f rightClipPoint);
  /**
   * @brief use the collision detector to check for a present collision
   * @return whether a collision is detected
   */
  bool collisionDetected() const;
  /**
   * @brief use the collision detector and rateKick to determine a free way to the opponent goal
   * @return whether the way to the opponent goal is free
   */
  bool isWayToGoalFree();
  /**
   * @brief use the collision detector and rateKick to determine a free way in the current interpolatedDirection
   * @return whether the interpolated way at the current teamBall position is free
   */
  bool isInterpolatedWayFree();
  /**
   * @brief check if the way in the given direction is free
   * @param direction the direction to be checked whether it is free
   * @return whether the way is free
   */
  bool isGivenWayFree(const Vector2f& direction);
  /**
   * @brief returns true if there is only one playing player
   * @return if the player is only playing team member
   */
  bool amIAlone() const;
  /**
   * @brief checks with hysteresis if the ball is near the own goal
   * @return if the ball is in the area near own goal
   */
  bool isBallNearOwnGoal();
  /**
   * @brief checks with hysteresis if the ball is near the opponent goal
   * @return if the ball is in the area near opponent goal
   */
  bool isBallNearOpponentGoal();
  /**
   * @brief check whether a side kick is useful. Currently false returning check as this is not implemented
   * @return if the side kick is useful
   */
  bool useSideKick() const;
  /**
   * @brief check whether a turn kick is useful. Currently false returning check as this is not implemented
   * @return if the turn kick is useful
   */
  bool useTurnKick() const;
  /**
   * @brief rate the position of a pass target to find the best target
   * @parameter position the position of the target
   * @return rating of the given passtarget
   */
  float ratePosition(const Vector2f& position) const;
  StrikerActionProvider::PassTarget findPassTarget(const float ballRating) const;
};
