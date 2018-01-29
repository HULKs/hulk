#pragma once

#include "Data/BallState.hpp"
#include "Data/MotionPlannerOutput.hpp"
#include "Data/MotionRequest.hpp"
#include "Data/ObstacleData.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Framework/Module.hpp"


// This forward declaration is needed for modules in our framework
class Brain;

/**
 * @brief MotionPlanner is responsible for determining the objective translation and rotation values to apply to the robot.
 *
 * Depending on the specified walking mode, obstacle avoidance may be performed.
 * Currently, a vector-based method of obstacle avoidance is used that works per-cycle.
 * All currently known obstacles are evaluated to determine the next waypoint towards a target position.
 *
 * @author Thomas Schattschneider
 */
class MotionPlanner : public Module<MotionPlanner, Brain>, public Uni::To
{
public:
  /// Constructor
  MotionPlanner(const ModuleManagerInterface& manager);

  /**
   * @brief cycle Calculates the objective translation and rotation values to apply to the robot, according to the chosen walking mode.
   */
  void cycle();

  /**
   * @brief toValue converts this to a Uni::Value
   *
   * This serializes all obstacles together with their avoidance radii
   * to enable proper visualization of the obstacle configuration space.
   * @param value the resulting Uni::Value
   */
  void toValue(Uni::Value& value) const;

private:
  /// If true, allow usage of different walking modes. Else, always fall back to path mode.
  const Parameter<bool> enableWalkingModes_;
  /// when within this distance, start interpolating between facing the target and aligning with target orientation [m]
  const Parameter<float> hybridAlignDistance_;
  /// when within this distance, align with target orientation [m]
  const Parameter<float> targetAlignDistance_;
  /// offset walk target will be shifted by this angle along the ball avoidance radius
  Parameter<float> ballOffsetShiftAngle_;
  /// The obstacle weight weighs the influence of the superposed obstacle displacement vector.
  const Parameter<float> obstacleWeight_;
  /// angle of the repelling force exerted by the ball obstacle [deg]
  Parameter<float> ballDisplacementAngle_;
  /**
   * Avoidance only occurs if the obstacle is within this radius to the robot [m].
   * Note that this is different than the physical obstacle radius which is stored in
   * the obstacle object itself. To the motion planner, only the avoidance radius matters.
   */
  Parameter<float> ballAvoidanceRadius_;
  /// weight factor of the ball obstacle, determines the strength of the repelling force
  Parameter<float> ballWeight_;
  /// angle of the repelling force exerted by obstacles from sonar detection [deg]
  Parameter<float> sonarDisplacementAngle_;
  /**
   * Avoidance only occurs if the obstacle is within this radius to the robot [m].
   * Note that this is different than the physical obstacle radius which is stored in
   * the obstacle object itself. To the motion planner, only the avoidance radius matters.
   */
  Parameter<float> sonarAvoidanceRadius_;
  /// weight factor of obstacles created by sonar detection, determines the strength of the repelling force
  Parameter<float> sonarWeight_;

  // Dependencies
  /// The motionRequest is used to get the position of the target
  const Dependency<MotionRequest> motionRequest_;
  /// The obstacleData gives access to all currently known obstacles.
  const Dependency<ObstacleData> obstacleData_;
  /// Provides coordinate transformations and robot pose in world coordinates
  const Dependency<RobotPosition> robotPosition_;
  /// The BallState is needed for specific motionplanning when walking behind the ball
  const Dependency<BallState> ballState_;

  // Production
  /// The output holds the waypoint as an x,y-position in robot coordinates
  Production<MotionPlannerOutput> motionPlannerOutput_;

  // State members
  bool offsetBallTargetReached_;

  /**
   * @brief Set a waypoint position pulled back from the ball, and after reaching it, set the target to the ball.
   *
   * This checks if the robot is outside config-defined angular region behind the ball with respect to the
   * direction pointing to the goal. If it is, it creates an offset target position away from the ball as a waypoint
   * to facilitate walking around the ball without touching it accidentally and for already aiming towards
   * walking around the ball even when the robot approaches from farther away. For more details check inline
   * comments and https://github.com/HULKs/nao/wiki/MotionPlanning.
   */
  void setWalkBehindBallPosition(float offsetRotationAngle);
  /**
   * @brief Determine objective rotation angle
   *
   * When using a walking mode that has fixed orientation (...WITH_ORIENTATION modes), just return
   * the target pose orientation as the objective rotation angle.
   *
   * Otherwise, interpolate between facing the target and adopting target orientation, depending on distance.
   *
   * @return Orientation to apply [rad]
   */
  float calculateRotation() const;
  /**
   * @brief Determines a vector for translational movement
   *
   * When using a walking mode that avoids obstacles (PATH... modes), this calculates a displacement vector
   * for each obstacle by calling displacementVector() for each one. All the displacement vectors are
   * normalized and superposed together with a vector pointing to the target, to obtain a objective direction.
   *
   * Otherwise, just return a noralized vector pointing to the target position as objective direction.
   *
   * @return Translation vector [m]
   */
  Vector2f calculateTranslation();
  /**
   * @brief Compute a displacement vector for "pushing" the robot around an obstacle
   * @return The vector representing the displacement caused by an obstacle
   */
  Vector2f displacementVectorOf(const Obstacle& obstacle) const;

  // Helper functions for getting the corresponding values for each obstacle type
  /**
   * @brief Return the displacement angle of an obstacle corresponding to its type
   * @param obstacle The obstacle to get the displacement angle of
   * @return The displacement of the obstacle
   */
  float displacementAngleOf(const Obstacle& obstacle) const;
  /**
   * @brief Return the avoidance radius of an obstacle corresponding to its type
   * @param obstacle The obstacle to get the avoidance radius of
   * @return The avoidance radius of the obstacle
   */
  float avoidanceRadiusOf(const Obstacle& obstacle) const;
  /**
   * @brief Return the weight of an obstacle corresponding to its type
   * @param obstacle The obstacle to get the weight of
   * @return The weight of the obstacle
   */
  float weightOf(const Obstacle& obstacle) const;

  /**
   * @brief Calculates the obstacleDisplacement vector. This vector represent the repulsive effect of obstacles.
   * This is used to create the "force" to push the robot away from obstacles.
   * @return obstacleDisplacement with some weighting, as normalized.
   */
  Vector2f getObstacleAvoidanceVector() const;

  //
  /**
   * @brief Interpolate between facing the target and adopting the target orientation.
   * @return interpolatedAngle
   */
  float getInterpolateAngle() const;
};


class ObstacleTypeError : public std::runtime_error
{
public:
  ObstacleTypeError()
    : std::runtime_error("Obstacle type is not correctly defined.")
  {
  }
};
