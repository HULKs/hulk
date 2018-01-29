#pragma once

#include <Data/MotionPlannerOutput.hpp>
#include <Framework/Module.hpp>
#include <Tools/Math/Pose.hpp>

/**
 * @enum supportFoot
 * @brief The supportFoot enum provides the support foot
 */
enum supportFoot
{
  SF_RIGHT_SUPPORT = -1,
  SF_DOUBLE_SUPPORT = 0,
  SF_LEFT_SUPPORT = 1,
  SF_NO_SUPPORT = 2
};

/**
 * @brief The StepPlanner calculates the next step position and orientation and returns it as a pose.
 *
 * The step planning is done in a way that ensures properly reaching
 * the target while utilizing the robots' movement capabilities efficiently,
 * e. g. moving and rotating at high speeds.
 *
 * @note Completely refactored by Thomas Schattschneider (TSchattschneider) in June, 2017.
 */
class StepPlanner
{
public:
  /// Constructor
  StepPlanner(const ModuleBase& module, const MotionPlannerOutput& motionPlannerOutput);

  /**
   * @brief Provides the position and rotation of the next step on the way to the target pose.
   *
   * Calculation of the next step pose is done by looking at the pose of the currently
   * active step and the target pose and figuring out how the next step has to be taken
   * in order to properly reach the target pose. Here, two cases are are taken
   * into account: First, the robot has to start decelerating early enough
   * to smoothly come to a stop when reaching the target pose. Secondly, it is
   * desirable to always move at maximum speeds when possible, which means utilizing
   * the preconfigured limits of the rotational and translational movements.
   *
   * Several checks are and adjustments to the step calculations are performed
   * in succession, first for the rotational movement, then for the translational movement,
   * to ensure a trade-off between braking in time and properly using the robots'
   * movement capabilities.
   *
   * @note  This can be extended later for supporting different walking modes,
   *        like facing the target, walking with fixed orientation, etc.
   *
   * @param currentStep Pose of the step that is actively being or has just been performed
   * @param currentSupport Current support foot, left or right
   * @param pendulumPeriodDuration The period duration of the pendulum model used for walking
   * @return Pose of the next step to be performed
   */
  Pose nextStep(const Pose& currentStep, const supportFoot currentSupport, const float pendulumPeriodDuration);

private:
  /// the output of the motion planner, necessary to access the next waypoint to go to
  const MotionPlannerOutput& motionPlannerOutput_;

  /*
    Configuration parameters
  */
  /// the amount by which the step length gets adjusted when necessary [m per step]
  const Parameter<float> stepLengthChange_;
  /// the threshold for minimum step length [m]
  const Parameter<float> stepLengthThreshold_;

  // All the angular values are defined in degrees in the configuration files for readability,
  // but are converted and used as radian values internally.
  /// The amount by which the rotational movement gets adjusted when necessary [deg per step]
  Parameter<float> rotationAngleChange_;
  /// The limit for rotational movement [deg]
  Parameter<float> rotationAngleLimit_;
  /// the threshold for the minimum rotation that is allowed to be performed [deg]
  Parameter<float> rotationAngleThreshold_;
  /// The linear velocity of the robot, used for determining the limit for maximum possible step length [m/s]
  const Parameter<float> linearVel_;
};
