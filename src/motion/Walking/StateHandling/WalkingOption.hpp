#pragma once

#include "Modules/Poses.h"
#include "WalkManState.hpp"
#include "WalkOptionInterface.hpp"

/**
 * @brief WalkingOption handles the state transitions during walking (!standing). Tranlates the
 * external request to the interface of the WalkGeneator.
 */
class WalkingOption : public WalkOptionInterface<void>
{
public:
  WalkingOption()
  {
    walkingState_ = WalkingState::WALKING_WITH_VELOCITY;
  }

  void transition(const WalkManState& wmState);
  void action(WalkManState& wmState);

protected:
  enum class WalkingState
  {
    WALKING_WITH_VELOCITY,
    WALK_TO_TARGET
  };
  /// the state of this suboption
  WalkingState walkingState_;
};

void WalkingOption::transition(const WalkManState& wmState)
{
  // the walking transitions (a common transition, does not depend on the previous state)
  if (wmState.motionPlannerOutput.walkData.mode == WalkMode::VELOCITY)
  {
    walkingState_ = WalkingState::WALKING_WITH_VELOCITY;
  }
  else
  {
    walkingState_ = WalkingState::WALK_TO_TARGET;
  }
}

void WalkingOption::action(WalkManState& wmState)
{
  // the walking action
  const Velocity& motionPlannerVelocity = wmState.motionPlannerOutput.walkData.velocity;
  const Pose& motionPlannerTarget = wmState.motionPlannerOutput.walkData.target;
  const Pose& finalTarget =
      Pose(motionPlannerTarget.position, wmState.motionRequest.walkData.target.orientation);
  const Pose& physicalMaxSpeed = wmState.walkGenerator.maxSpeed;
  switch (walkingState_)
  {
    case WalkingState::WALKING_WITH_VELOCITY:
    {
      Velocity absoluteVelocity = Velocity();
      if (motionPlannerVelocity.isPercentage())
      {
        absoluteVelocity =
            Velocity(motionPlannerVelocity.translation * physicalMaxSpeed.position.x(),
                     motionPlannerVelocity.rotation * physicalMaxSpeed.orientation, false);
      }
      else
      {
        absoluteVelocity = motionPlannerVelocity;
      }
      wmState.setWalkParametersForVelocityMode(absoluteVelocity);
      break;
    }
    case WalkingState::WALK_TO_TARGET:
    {
      // in target mode, the velocity points towards the direction we want to walk
      const Pose walkDirection =
          Pose(motionPlannerVelocity.translation.normalized(), motionPlannerTarget.orientation);
      float requestedTranslationVelocity = 0.f;

      // figure out the absolute velocity request for the WalkGenerator
      Velocity velocityComponentLimits = Velocity();
      // if in percentage mode, we need to convert to absolute velocities
      if (motionPlannerVelocity.isPercentage())
      {
        // convert the percentage to an absolute velocity request (scalar)
        const float translationPercentage = motionPlannerVelocity.translation.norm();
        const float rotationPercentage = std::abs(motionPlannerVelocity.rotation);
        requestedTranslationVelocity = physicalMaxSpeed.position.x() * translationPercentage;
        velocityComponentLimits =
            Velocity(translationPercentage * physicalMaxSpeed.position,
                     rotationPercentage * physicalMaxSpeed.orientation, false);
      }
      else
      {
        // if the velocity of the motion planner was not given as a percentage, then the length of
        // the velocity vector determines the absolute speed. e.g. if veolcity.translation.norm() ==
        // 0.05 then we want to walk with 0.05m/s. In this case the velocity does NOT encode any
        // direction information
        requestedTranslationVelocity = motionPlannerVelocity.translation.norm();
        velocityComponentLimits = motionPlannerVelocity;
      }

      // the walk gradient vector (direction with magnitude containing the velocity information)
      // is assembled from the MotionPlannerOuput
      const Pose walkPathGradient =
          Pose(motionPlannerVelocity.translation.normalized() * requestedTranslationVelocity,
               motionPlannerTarget.orientation);
      wmState.setWalkParametersForTargetMode(velocityComponentLimits, finalTarget,
                                             walkPathGradient);
      break;
    }
  }
}
