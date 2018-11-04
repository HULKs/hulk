#include <boost/math/special_functions/sign.hpp>
#include <cmath>

#include "Data/MotionPlannerOutput.hpp"
#include "Modules/NaoProvider.h"
#include "Tools/Math/Range.hpp"

#include "StepPlanner.hpp"


using boost::math::sign; // Shortens long function calls

StepPlanner::StepPlanner(const ModuleBase& module, const MotionPlannerOutput& motionPlannerOutput)
  : motionPlannerOutput_(motionPlannerOutput)
  , stepLengthChange_(module, "stepLengthChange", [] {})
  , stepLengthThreshold_(module, "stepLengthThreshold", [] {})
  , rotationAngleChange_(module, "rotationAngleChange", [this] { rotationAngleChange_() *= TO_RAD; })
  , rotationAngleLimit_(module, "rotationAngleLimit", [this] { rotationAngleLimit_() *= TO_RAD; })
  , rotationAngleThreshold_(module, "rotationAngleThreshold", [this] { rotationAngleThreshold_() *= TO_RAD; })
  , linearVel_(module, "linearVel", [] {})
{
  rotationAngleChange_() *= TO_RAD;
  rotationAngleLimit_() *= TO_RAD;
  rotationAngleThreshold_() *= TO_RAD;
}

Pose StepPlanner::nextStep(const Pose& currentStep, const supportFoot currentSupport, const float pendulumPeriodDuration)
{
  const Pose& targetPose = motionPlannerOutput_.walkData.target;
  const bool velocityModeActive = (motionPlannerOutput_.walkData.mode == WalkMode::VELOCITY);
  // Calculate the rotation for the next step. Several checks
  // are performed to ensure that the robot starts braking its rotational movement
  // early enough in order to properly come to a stop in time.

  // 1. Rotation steps must not exceed the maximum angle.
  Range<float> aRange(-rotationAngleLimit_(), rotationAngleLimit_());

  // 2. Get the orientation that should be achieved
  float desiredRotation;
  if (velocityModeActive)
  {
    // In velocity mode, the specified velocity is the same as the desired rotation for one step (after proper conversion)
    desiredRotation = motionPlannerOutput_.walkData.velocity.rotation;
    if (motionPlannerOutput_.walkData.velocity.isPercentage())
    {
      // Convert percentage into fraction of maximum rotation speed
      desiredRotation *= rotationAngleLimit_();
    }
    else
    {
      // Convert [rad per second] into [rad per step]
      desiredRotation *= pendulumPeriodDuration;
    }
  }
  else
  {
    // When not in velocity mode, the desired rotation is given by the target orientation, but must sill adhere to specified velocity limits.
    desiredRotation = motionPlannerOutput_.walkData.target.orientation;
    // 2a. Rotation steps must not exceed the specified angle velocity.
    float rotationVelocity = motionPlannerOutput_.walkData.velocity.rotation;
    if (motionPlannerOutput_.walkData.velocity.isPercentage())
    {
      // Convert percentage into fraction of maximum rotation speed
      rotationVelocity *= rotationAngleLimit_();
    }
    else
    {
      // Convert [rad per second] into [rad per step]
      rotationVelocity *= pendulumPeriodDuration;
    }
    aRange.intersect(-rotationVelocity, rotationVelocity);
  }
  // 3. A step must not be much larger or smaller than the previous one.
  aRange.intersect(currentStep.orientation - rotationAngleChange_(), currentStep.orientation + rotationAngleChange_());
  // 4. Braking should still be possible when not in velocity mode.
  if (!velocityModeActive && std::abs(desiredRotation) > rotationAngleThreshold_())
  {
    // k is an (exact) guess of the number of steps that is needed to reach the target.
    const float k = std::ceil(0.5f * (1.f + std::sqrt(1.f + 8.f * std::abs(desiredRotation) / rotationAngleChange_()))) - 1.f;
    // Given the number of steps, the maximum step rotation can be calculated directly.
    const float brake = std::abs(desiredRotation) / k + 0.5f * rotationAngleChange_() * (k - 1.f);
    aRange.intersect(-brake, brake);
  }
  // 5. Make either a direct step to the desired rotation or choose the step that comes closest.
  aRange.intersect(desiredRotation, desiredRotation);
  // 6. The range may contain only one element now which becomes the rotation of the step.
  assert(aRange.min == aRange.max);
  float stepRotation = aRange.min;
  // 7. Too small rotations are clipped to zero.
  // rotationAngleThreshold_ should be small enough that its effect on keeping the other constraint is negligible.
  if (std::abs(stepRotation) < rotationAngleThreshold_())
  {
    stepRotation = 0.f;
  }

  // Calculate as a percentage how much of the rotational angle limit is "used up" by the step rotation.
  // This determines how much translational motion can still be performed together with the rotation.
  //
  // Example: rotationAngleLimit is 20°, stepRotation is 15°.
  // This means that 75% of the rotational limit is being used,
  // and 25% of the desired translational motion can be performed.
  float desiredAlignPercentage = std::abs(stepRotation / rotationAngleLimit_());
  float resultingStepPercentage = 1 - desiredAlignPercentage;

  const float maximumPossibleStepLength = pendulumPeriodDuration * linearVel_();
  const float distanceToTargetPose = targetPose.position.norm();
  Vector2f resultingStepPosition;
  // do-while-loop for structural goto
  do
  {
    // 1. The maximum velocity must not be exceeded (not factoring in resultingStepPercentage yet).
    float rCentered = maximumPossibleStepLength;
    // 2. A step must not be much larger or smaller than the previous one.
    const Vector2f outer = currentStep.position;
    const float outerAbs = outer.norm();
    const float rOuter = stepLengthChange_();
    if (outerAbs > rCentered + rOuter)
    {
      resultingStepPosition = outer * rCentered / outerAbs;
      break;
    }
    // 3. Now resultingStepPercentage is considered (this can make the centered circle only smaller).
    rCentered = maximumPossibleStepLength * resultingStepPercentage;
    if (outerAbs > rCentered + rOuter)
    {
      resultingStepPosition = outer * (1.f - rOuter / outerAbs);
      break;
    }
    // 4. Specifically requested velocity must not be exceeded
    float translationVelocity = motionPlannerOutput_.walkData.velocity.translation.norm();
    if (motionPlannerOutput_.walkData.velocity.isPercentage())
    {
      // Convert percentage to step length
      translationVelocity *= maximumPossibleStepLength;
    }
    else
    {
      // Convert meters per second to meters per step
      translationVelocity *= pendulumPeriodDuration;
    }
    rCentered = std::min(rCentered, translationVelocity);
    if (outerAbs > rCentered + rOuter)
    {
      resultingStepPosition = outer * (1.f - rOuter / outerAbs);
      break;
    }
    // 5. Braking should still be possible when not in velocity mode.
    if (!velocityModeActive && distanceToTargetPose > stepLengthThreshold_())
    {
      // k is an (exact) guess of the number of steps that is needed to reach the target.
      const float k = std::ceil(0.5f * (1.f + std::sqrt(1.f + 8.f * distanceToTargetPose / stepLengthChange_()))) - 1.f;
      // Given the number of steps, the maximum step length can be calculated directly.
      const float brake = distanceToTargetPose / k + 0.5f * stepLengthChange_() * (k - 1.f);
      rCentered = std::min(rCentered, brake);
      if (outerAbs > rCentered + rOuter)
      {
        resultingStepPosition = outer * (1.f - rOuter / outerAbs);
        break;
      }
    }
    // 6. Intersect waypoint (direction) with the disc intersection.
    // This gives a line segment or a single point
    Vector2f direction = motionPlannerOutput_.walkData.velocity.translation;
    direction.normalize();
    // r1 and r2 are the radii of the line segment.
    float r1 = 0.f, r2 = 0.f;
    if (outerAbs + rOuter <= rCentered)
    {
      // This is the case that the outer circle is completely enclosed in the centered circle.
      // This case is medium.
      // Determine the intersections of the direction ray and the outer circle.
      const float np_2 = outer.dot(direction);
      const float q = outerAbs * outerAbs - rOuter * rOuter;
      const float radicand = np_2 * np_2 - q;
      const float radix = radicand > 0.f ? std::sqrt(radicand) : 0.f;
      if (radicand <= 0.f || (np_2 + radix) < 0.f)
      {
        // No intersection, one intersection or two intersections but both are behind the ray.
        // projectionFromOuter is the point on the direction ray that is closest to the outer circle (relative to the center of the outer circle).
        const Vector2f projectionFromOuter = direction * std::max(np_2, 0.f) - outer;
        // This is the point on the boundary of the outer circle that is closest to the direction ray.
        resultingStepPosition = outer + projectionFromOuter * rOuter / projectionFromOuter.norm();
        break;
      }
      // This case means that there is at least one intersection of the circle with the direction ray.
      r1 = std::max(np_2 - radix, 0.f);
      r2 = np_2 + radix;
    }
    else if (outerAbs + rCentered <= rOuter)
    {
      // This is the case that the centered circle is completely enclosed in the outer circle.
      // This case is easy.
      // r1 is already zero.
      r2 = rCentered;
    }
    else
    {
      // Both circles intersect at least in one point.
      // This case is hard.
      // Determine the intersections of the direction ray and the outer circle.
      const float np_2 = outer.dot(direction);
      const float q = outerAbs * outerAbs - rOuter * rOuter;
      const float radicand = np_2 * np_2 - q;
      const float radix = radicand > 0.f ? std::sqrt(radicand) : 0.f;
      if (radicand <= 0.f || (np_2 + radix) < 0.f)
      {
        // No intersection, one intersection or two intersections but both are behind the ray.
        const Vector2f projectionFromOuter = direction * std::max(np_2, 0.f) - outer;
        resultingStepPosition = outer + projectionFromOuter * rOuter / projectionFromOuter.norm();
        if (resultingStepPosition.norm() < rCentered)
        {
          break;
        }
        const float a = (rCentered * rCentered - rOuter * rOuter + outerAbs * outerAbs) / (2.f * outerAbs);
        const float x2 = outer.x() * a / outerAbs;
        const float y2 = outer.y() * a / outerAbs;
        const float h = std::sqrt(rCentered * rCentered - a * a);
        const float rx = -outer.y() * (h / outerAbs);
        const float ry = outer.x() * (h / outerAbs);
        const Vector2f i1 = Vector2f(x2 + rx, y2 + ry);
        const Vector2f i2 = Vector2f(x2 - rx, y2 - ry);
        if ((i1 - resultingStepPosition).norm() < (i2 - resultingStepPosition).norm())
        {
          resultingStepPosition = i1;
        }
        else
        {
          resultingStepPosition = i2;
        }
        break;
      }
      const float rIntersection1 = std::max(np_2 - radix, 0.f);
      const float rIntersection2 = np_2 + radix;
      // TODO: Was commented out
      // const Vector2f centeredIntersection = direction * rCentered;
      if (rIntersection1 <= rCentered)
      {
        r1 = rIntersection1;
        r2 = std::min(rIntersection2, rCentered);
      }
      else
      {
        const float a = (rCentered * rCentered - rOuter * rOuter + outerAbs * outerAbs) / (2.f * outerAbs);
        const float x2 = outer.x() * a / outerAbs;
        const float y2 = outer.y() * a / outerAbs;
        const float h = std::sqrt(rCentered * rCentered - a * a);
        const float rx = -outer.y() * (h / outerAbs);
        const float ry = outer.x() * (h / outerAbs);
        const Vector2f i1 = Vector2f(x2 + rx, y2 + ry);
        const Vector2f i2 = Vector2f(x2 - rx, y2 - ry);
        if ((i1 - direction * rIntersection1).norm() < (i2 - direction * rIntersection1).norm())
        {
          resultingStepPosition = i1;
        }
        else
        {
          resultingStepPosition = i2;
        }
        break;
      }
    }
    assert(r1 <= r2);
    // 7. Use distance to target to dermine final step length.
    // This means that if the waypoint does not point in the direction of the target only small steps are made in the vincinity of the target.
    // But there is no better behavior I can think of as long as waypoint and target are coexisting.
    if (velocityModeActive || distanceToTargetPose > r2)
    {
      resultingStepPosition = direction * r2;
    }
    else if (distanceToTargetPose < r1)
    {
      resultingStepPosition = direction * r1;
    }
    else
    {
      resultingStepPosition = direction * distanceToTargetPose;
    }
  } while (false);

  // Calculate as a percentage how much of the physically possible maximum step length is actually being used.
  if (maximumPossibleStepLength == 0)
  {
    resultingStepPercentage = 0;
  }
  else
  {
    resultingStepPercentage = std::abs(resultingStepPosition.norm() / maximumPossibleStepLength);
  }

  // If at this point the sum of rotational and translational movement percentages is greater than 100%,
  // it means that the step length couldn't be limited as required. So instead, limit the step rotation again.
  if (resultingStepPercentage + desiredAlignPercentage > 1)
    stepRotation = rotationAngleLimit_() * (1 - resultingStepPercentage) * sign(stepRotation);

  // Avoid moving towards the support leg
  if ((currentSupport == SF_LEFT_SUPPORT && resultingStepPosition.y() > 0) || (currentSupport == SF_RIGHT_SUPPORT && resultingStepPosition.y() < 0))
  {
    resultingStepPosition.y() = 0;
  }


  return Pose(resultingStepPosition, stepRotation);
}
