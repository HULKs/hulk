#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/DribbleData.hpp"
#include "Data/ObstacleData.hpp"
#include "Data/PathPlannerData.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/StepPlan.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamObstacleData.hpp"
#include "Data/WalkGeneratorOutput.hpp"
#include "Framework/Module.hpp"


class Motion;

/**
 * StepPlanner takes the requests regarding walking from Brain and translates it to a step request,
 * which can be executed by the WalkGenerator
 */
class StepPlanner : public Module<StepPlanner, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"StepPlanner"};

  explicit StepPlanner(const ModuleManagerInterface& manager);

  /**
   * @brief cycle Calculates the objective translation and rotation values to apply to the robot,
   * according to the chosen walking mode.
   */
  void cycle() override;

private:
  // Dependencies
  /// The action command is used to get the position of the target
  const Dependency<ActionCommand> actionCommand_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<RobotPosition> robotPosition_;
  /// the path planned by brain
  const Dependency<PathPlannerData> pathPlannerData_;
  /// the cycle information of this cycle
  const Dependency<CycleInfo> cycleInfo_;
  /// used for obstacle avoidance
  const Dependency<TeamObstacleData> teamObstacleData_;
  /// used to walk around the ball safely
  const Dependency<TeamBallModel> teamBallModel_;
  /// handle step requests for DRIBBLE mode
  const Dependency<DribbleData> dribbleData_;
  /// information about the walking from the last cycle
  const Reference<WalkGeneratorOutput> walkGeneratorOutput_;

  // Parameters
  /// How much of rotation is done by turning feet to the inside (0..1)
  const Parameter<float> insideTurnRatio_;
  /// Maximum acceleration of forward and sideways speed [m/step/step]
  const Parameter<float> maxForwardAcceleration_;
  /// parametrize the walk volume; forms the shape for diagonal steps
  const Parameter<float> walkVolumeTranslationExponent_;
  /// parametrize the walk volume; scales the influence of the turn component
  const Parameter<float> walkVolumeRotationExponent_;
  /// Maximum stepsize [m/step] and [deg/step]
  Parameter<Pose> maxStepSize_;
  /// Maximum backwards step size. positive; [m/step]
  const Parameter<float> maxStepSizeBackwards_;

  /// The output of this module
  Production<StepPlan> stepPlan_;

  /**
   * @brief calculates the pose the robot should walk in the requested walk mode
   * @return the relative robot pose the robot should approach
   */
  Pose calculateNextPose() const;

  /**
   * @brief subtracts the return offset of the current step from the requested target walking
   * should reach. The return offset represents the distance the robot's torso moves anyway when
   * it stops now.
   * @param request the target walking to [m] and [rad]
   * @return the compensated target [m] and [rad]
   */
  Pose compensateWithReturnOffset(const Pose& request) const;

  /**
   * @brief clamps the requested step sizes with the maximum allowed acceleration and deceleration
   * @param request the requested step sizes [m] and [rad]
   * @return the clamped step sizes [m] and [rad]
   */
  Pose clampAcceleration(const Pose& request) const;

  /**
   * @brief calculates the walk volume. This is a measure how "big" the requested step is with
   * regards to the configured and physical limits. This is used to evaluate whether a step is
   * feasible. Any value <= 1 represents an executable volume.
   * @param forward the requested step size in forward direction [m]
   * @param left the requested step size in sideways direction [m]
   * @param turn the turn amount of this step. Should not exceed the maximum limit maxTurn [rad]
   * @param maxForward the maximum forward step size [m]
   * @param maxBackwards the maximum backwards step size [m]
   * @param maxSideways the maximum sideways step size [m]
   * @param maxTurn the maximum turn step size [rad]
   * @return the walk volume
   */
  float calculateWalkVolume(float forward, float left, float turn, float maxForward = 1.f,
                            float maxBackwards = 1.f, float maxSideways = 1.f,
                            float maxTurn = 1.f) const;

  /**
   * @brief calculates the maximum translational step sizes for a given turn in this step and
   * clamps the given step sizes forward and left to the maximum feasible size.
   * @param forward the requested step size in forward direction [m]
   * @param left the requested step size in sideways direction [m]
   * @param turn the turn amount of this step. Should not exceed the maximum limit maxTurn [rad]
   * @param maxForward the maximum forward step size [m]
   * @param maxBackwards the maximum backwards step size [m]
   * @param maxSideways the maximum sideways step size [m]
   * @param maxTurn the maximum turn step size [rad]
   * @return the clamped translational step sizes
   */
  Vector2f calculateMaxStepSizeInWalkVolume(float forward, float left, float turn,
                                            float maxForward = 1.f, float maxBackwards = 1.f,
                                            float maxSideways = 1.f, float maxTurn = 1.f) const;

  /** @brief clamps a given step size request to its maximum feasible step size using the walk
   * volume
   * @param maxStepSize the componentwise step size limits (maximum sizes) [m] and [rad]
   * @param maxStepSizeBackwards the maximum step size in backwards direction [m]
   * @param targetPose the target to clamp
   * @return the clamped pose representing the step size to execute
   */
  Pose clampStepToWalkVolume(const Pose& maxStepSize, float maxStepSizeBackwards,
                             const Pose& targetPose) const;

  /**
   * @brief clampToAnatomicConstraints takes the current walk phase into account to only request
   * anatomically possible steps
   * @param request the target request
   * @return the clamped request
   */
  Pose clampToAnatomicConstraints(const Pose& request);
};
