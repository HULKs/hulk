#include <cmath>

#include "FootController.hpp"
#include <Framework/Module.hpp>
#include <Tools/Kinematics/InverseKinematics.h>
#include <Tools/Math/Angle.hpp>

/** Constructor **/
FootController::FootController(const ModuleBase& module, const IMUSensorData& imuSensorData)
  : setDynamicSteps_(module, "setDynamicSteps", [] {})
  , lowPassAlphaDynamicSteps_(module, "lowPassAlphaDynamicSteps", [] {})
  , stepHeight_(module, "stepHeight", [] {})
  , sideStepHeight_(module, "sideStepHeight", [] {})
  , imuSensorData_(imuSensorData)
{
}

/** Pendulum::step **/
void FootController::getStep(const float progress, FootPose3D& currentFootPose, const Step2D& targetFootPose, const Step2D& lastFootPose,
                             InWalkKickType kickType, float& maxImuError, float& maxLastImuError, float& dynamicStepAccumulator)
{
  /// This function calculates the position of the swinging foot taking step
  /// commands into account
  // On artificial turf a higher steps are required for turning and side steps

  float swingSafetyStep = 0.f;
  // For comparison of old an new walking
  if (setDynamicSteps_())
  {
    float foot2GroundAngle = imuSensorData_.angle.y();
    /**
     * The main problem with longer walking distances is, that the robort start to swing after a while.
     * Thus the maximum of the IMU-Angle of the current step is stored.
     */
    maxImuError = std::max(maxImuError, std::fabs(foot2GroundAngle));
    float imuError = std::max(maxImuError, maxLastImuError);

    // lowPassFilter for safety currentFootPose:
    // Filter sensor data for less vibration:
    dynamicStepAccumulator = lowPassAlphaDynamicSteps_() * imuError + (1 - lowPassAlphaDynamicSteps_()) * dynamicStepAccumulator;
    // Movement in the plane should be only done after the foot has reached a certain height / before it goes below that height.

    swingSafetyStep = std::sin(dynamicStepAccumulator) * targetFootPose.position.x();
  }

  // Calculate dynamic step height depending on the walking direction
  float dynamicStepHeight = calculateCurrentStepHeight(lastFootPose, targetFootPose) + swingSafetyStep;

  float interpolationFactor = (1 - cos(progress * M_PI)) / 2;
  // Lift the foot according to a cos function that has its stationary points at 0 (minimum), 0.5 (maximum) and 1 (minimum).
  currentFootPose.position.z() = (1 - cos(progress * 2 * M_PI)) / 2 * dynamicStepHeight;

  // Interpolate between old currentFootPose and current currentFootPose.
  currentFootPose.position.x() = lastFootPose.position.x() + (targetFootPose.position.x() - lastFootPose.position.x()) * interpolationFactor;
  currentFootPose.position.y() = lastFootPose.position.y() + (targetFootPose.position.y() - lastFootPose.position.y()) * interpolationFactor;
  currentFootPose.orientation = lastFootPose.orientation + (targetFootPose.orientation - lastFootPose.orientation) * interpolationFactor;

  if (kickType != InWalkKickType::NONE)
  {
    Vector3f footForcingTerm = getFootForcingTerm(progress, kickType);
    // apply the foot forcing term by combining the element whise max:
    // TODO: This is not a good idea since this foot migth be placed backwards
    currentFootPose.position = currentFootPose.position + footForcingTerm;
  }
}

Vector3f FootController::getFootForcingTerm(const float progress, InWalkKickType kickType)
{
  // For proof of concept only add a static forcing term to the foots x-trajectory:
  // This could longtermingly even be some sort of DMP stuff
  float xMax = 0.f;
  float zMax = 0.f;
  // TODO: Make this configurable
  if (kickType == InWalkKickType::RIGHT_GENTLE || kickType == InWalkKickType::LEFT_GENTLE)
  {
    xMax = 0.035;
    zMax = 0.015;
  }
  else if (kickType == InWalkKickType::RIGHT_STRONG || kickType == InWalkKickType::LEFT_STRONG)
  {
    xMax = 0.05;
    zMax = 0.02;
  }
  float shootForcingX = (1 - cos(progress * 2 * M_PI)) / 2 * xMax;
  float shootForcingZ = sin(progress * M_PI) * zMax;
  // Add forcing term onto original trajectory

  return {shootForcingX, 0, shootForcingZ};
}

float FootController::calculateCurrentStepHeight(const Step2D& lastFootPose, const Step2D& targetFootPose) const
{
  const Vector2f walkingDirection = targetFootPose.position - lastFootPose.position;
  if (walkingDirection.squaredNorm() < 0.001)
  {
    return stepHeight_();
  }
  const float absTargetAngle = std::abs(std::atan2(walkingDirection.y(), walkingDirection.x()));

  if (absTargetAngle < 90.f * TO_RAD)
  {
    // TODO:  reason about this
    float frontFraction = std::cos(absTargetAngle);
    return frontFraction * stepHeight_() + (1.f - frontFraction) * sideStepHeight_();
  }
  else
  {
    // Walking sidewards or backwards with individual stepheight
    return sideStepHeight_();
  }
}
