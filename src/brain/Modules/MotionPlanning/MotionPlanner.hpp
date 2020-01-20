#pragma once

#include "Data/BallState.hpp"
#include "Data/MotionPlannerOutput.hpp"
#include "Data/MotionRequest.hpp"
#include "Data/ObstacleData.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamObstacleData.hpp"
#include "Data/WalkingEngineWalkOutput.hpp"
#include "Framework/Module.hpp"


// This forward declaration is needed for modules in our framework
class Brain;

/**
 * @brief MotionPlanner is responsible for determining the objective translation and rotation values
 * to apply to the robot.
 *
 * Depending on the specified walking mode, obstacle avoidance may be performed.
 * Currently, a vector-based method of obstacle avoidance is used that works per-cycle.
 * All currently known obstacles are evaluated to determine the next waypoint towards a target
 * position.
 *
 * @author Thomas Schattschneider
 */
class MotionPlanner : public Module<MotionPlanner, Brain>, public Uni::To
{
public:
  /// the name of this module
  ModuleName name = "MotionPlanner";
  /// Constructor
  MotionPlanner(const ModuleManagerInterface& manager);

  /**
   * @brief cycle Calculates the objective translation and rotation values to apply to the robot,
   * according to the chosen walking mode.
   */
  void cycle();

  /**
   * @brief toValue converts this to a Uni::Value
   *
   * This serializes final translation and rotation along with the possibly modified walkTarget.
   * @param value the resulting Uni::Value
   */
  void toValue(Uni::Value& value) const;

private:
  enum class FootDecision
  {
    NONE,
    LEFT,
    RIGHT
  };

  /// when within this distance, start interpolating between facing the target and aligning with
  /// target orientation [m]
  const Parameter<float> hybridAlignDistance_;
  /// Only while dribbling and near to the walk target: specifies the distance when the robot should
  /// be fully aligned with the walk target pose orientation [m].
  const Parameter<float> dribbleAlignDistance_;
  /// specifies the distance when the robot should be fully aligned with the walk target pose
  /// orientation when it is not dribbling or far away from the walk target pose. [m]
  const Parameter<float> targetAlignDistance_;
  /// offset ball walk target will be shifted by this angle along the ball avoidance radius [deg]
  Parameter<float> ballOffsetShiftAngle_;
  /// The offset ball walk target will be pulled back from the ball by this distance [m]
  const Parameter<float> ballOffsetDistance_;
  /// the tolerance at which the ball offest target is to be reached
  Parameter<float> ballOffsetTargetOrientationTolerance_;

  // Obstacle parameters
  /// weight of the ball obstacle.
  const Parameter<float> ballWeight_;
  /// weight of the free kick area obstacle.
  const Parameter<float> freeKickAreaWeight_;
  /// weight of obstacles with robot type.
  const Parameter<float> robotWeight_;
  /// weight of obstacles with fallen robot type.
  const Parameter<float> fallenRobotWeight_;
  /// weight of obstacles with unknown obstacle type.
  const Parameter<float> unknownObstacleWeight_;
  /// The total obstacle weight modifies the influence of the completely superimposed obstacle
  /// displacement vector.
  const Parameter<float> totalObstacleWeight_;
  /// angle of the repelling force exerted by obstacles [deg]
  Parameter<float> obstacleDisplacementAngle_;

  /// set to true to make the striker only use the obstacles he saw himself
  const Parameter<bool> strikerUsesOnlyLocalObstacles_;
  /// set to true to ignore goal post obstacles in motion planning (since they might be dangerous in
  /// cases were they prevent the robot from reaching its target (e.g. ball near goal post)) set to
  const Parameter<bool> ignoreGoalPostObstacles_;
  /// set to true to use a different walking speed while dribbling
  const Parameter<bool> enableCarefulDribbling_;
  /// the factor applied to the translational velocity when dribbiling carefully
  const Parameter<float> carefulDribbleSpeed_;
  /// the distance to the ball at which we start carefully dribbling more carefully
  const Parameter<float> carefulDribbleDistanceThreshold_;
  /// Offset of robotfoot to ball while dribbling, in order to assure he hits ball with his foot
  const Parameter<float> footOffset_;
  /// the minimum distance that we can come close to the outer surface of an obstacle, if we can
  /// collide with this obstacle on foot height
  const Parameter<float> groundLevelAvoidanceDistance_;
  /// the minimum distance that we can come close to the outer surface of an obstacle, if we can
  /// collide with this obstacle on shoulder height
  const Parameter<float> shoulderLevelAvoidanceDistance_;
  /**
   * the tolerance describing how much the robots direction may deviate from
   * the desired dribbling direction without the need to reposition.
   */
  Parameter<float> dribblingAngleTolerance_;
  /// This is used when not dribbling to approach the ball more slowly (avoids overshoot)
  const Parameter<float> slowBallApproachFactor_;
  const Parameter<float> maxDistToBallTargetLine_;
  /// thresholds to decide when to walk around the ball
  const Parameter<float> walkAroundBallDistanceThreshold_;
  Parameter<float> walkAroudBallAngleThreshold_;

  // Dependencies
  /// The motionRequest is used to get the position of the target
  const Dependency<MotionRequest> motionRequest_;
  /// the ObstacleData gives access to the local obstacle model
  const Dependency<ObstacleData> obstacleData_;
  /// the TeamObstacleData gives access to all currently known obstacles (local model and obstacles
  /// of the team)
  const Dependency<TeamObstacleData> teamObstacleData_;
  /// Provides coordinate transformations and robot pose in world coordinates
  const Dependency<RobotPosition> robotPosition_;
  /// The BallState is needed for specific motion planning when walking behind the ball
  const Dependency<BallState> ballState_;
  /// the output of the walking module to figure out how fast we can walk
  const Dependency<WalkingEngineWalkOutput> walkingEngineWalkOutput_;
  /// the current role assignment - used to figure out whether we are striker
  const Dependency<PlayingRoles> playingRoles_;

  // Production
  /// The output holds the waypoint as an x,y-position in robot coordinates
  Production<MotionPlannerOutput> motionPlannerOutput_;

  // State members
  /// This array associates each obstacle type with a weight
  std::array<float, static_cast<int>(ObstacleType::OBSTACLETYPE_MAX)> obstacleWeights_;
  /// A flag indicating if the offset walk target has been reached
  bool offsetBallTargetReached_;
  /// A flag indicating if the walk target for walking around the ball has been reached
  bool walkAroundBallTargetReached_;
  /// A flag indicating if the ball obstace should be ignored during obstacle avoidance.
  bool ignoreBallObstacle_;
  /// A flag indicating if robot obstacles should be ignored during obstacle avoidance.
  bool ignoreRobotObstacles_;
  /// Documents the last foot decision and is than used to give a margin of error
  FootDecision lastfootdecision_;
  /// Counts the amounts of cycle to reduce update of foot decision
  unsigned int cycleCounter_ = 0;
  /// a pose used for walking around the ball in a circle while facing it
  Pose walkAroundBallPose_;

  /**
   * @brief Set a waypoint position pulled back from the ball, and after reaching it, set the target
   * to the ball.
   *
   * This checks if the robot is outside config-defined angular region behind the ball with respect
   * to the direction pointing to the goal. If it is, it creates an offset target position away from
   * the ball as a waypoint to facilitate walking around the ball without touching it accidentally
   * and for already aiming towards walking around the ball even when the robot approaches from
   * farther away. For more details check inline comments and
   * https://github.com/HULKs/nao/wiki/MotionPlanning.
   *
   * @param offsetRotationAngle Specifies how much the offset target is rotated towards the robot.
   */
  void setWalkBehindBallPosition(float offsetRotationAngle);
  /**
   * @brief Determine objective rotation angle
   *
   * When using a walking mode that has fixed orientation (...WITH_ORIENTATION modes), just return
   * the target pose orientation as the objective rotation angle.
   *
   * Otherwise, interpolate between facing the target and adopting target orientation, depending on
   * distance.
   *
   * @return Orientation to apply [rad]
   */
  float calculateRotation() const;
  /**
   * @brief Determines a vector for translational movement
   *
   * When using a walking mode that avoids obstacles (PATH... modes), this calculates a displacement
   * vector for each obstacle by calling displacementVector() for each one. All the displacement
   * vectors are normalized and superposed together with a vector pointing to the target, to obtain
   * a objective direction.
   *
   * Otherwise, just return a normalized vector pointing to the target position as objective
   * direction.
   *
   * @return Translation vector [m]
   */
  Vector2f calculateTranslation();
  /**
   * @brief Compute a single obstacle displacement vector for "pushing" the robot around the
   * obstacle
   * @return The vector representing the displacement caused by a single obstacle
   */
  Vector2f displacementVector(const Obstacle& obstacle) const;

  /**
   * @brief getClippedDribbleVelocity clips a given velocity to carefullDribbleSpeed
   * @param requestedVelocity the unclipped requested velocity
   * @return a clipped velocity object with the same translational direction
   */
  Velocity getClippedDribbleVelocity(const Velocity& requestedVelocity) const;

  /**
   * @brief Calculate a vector pointing to a position near the ball for dribbling.
   * @return The direction to walk to while dribbling as normalized vector.
   */
  Vector2f dribblingDirection();

  /**
   * @brief Calculates a final superimposed displacement vector, which represents the repulsive
   * effect of obstacles. This is used to to "push" the robot away from obstacles.
   * @return displacement vector normalized.
   */
  Vector2f obstacleAvoidanceVector() const;

  /**
   * @brief getRelevantObstacles returns a vector of pointers to the relevant obstacles
   * @return a vector of pointers to the relevant obstacles
   */
  std::vector<const Obstacle*> getRelevantObstacles() const;

  /**
   * @brief Interpolate between facing the target and adopting the target orientation.
   * @return interpolatedAngle
   */
  float interpolatedAngle(const float targetAlignDistance) const;
  /**
   * @brief getMinDistToObstacleCenter figures out how close we can come to an obstacle. This
   * considers the height of the collision (e.g. with ball we can only collide on foot height, not
   * with the shoulders) and adds an according avoidance distance to the obstacle radius
   * @param obstacle the obstacle to get the min distance for
   * @return the minimum distance that we should come close the the center of the given obstacle
   * (with our cener)
   */
  float getMinDistToObstacleCenter(const Obstacle& obstacle) const;
};
