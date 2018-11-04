#pragma once

#include "InWalkKickOption.hpp"
#include "Modules/Poses.h"
#include "WalkManState.hpp"
#include "WalkOptionInterface.hpp"
#include "WalkingOption.hpp"

/**
 * @brief ActiveOption handles the state while the walking output is active. This handles the
 * transition between walking and standing. In case of standing, stand is directly requested. In
 * case of walking the WalkingOption is called as a suboption.
 */
class ActiveOption : public WalkOptionInterface<void>
{
public:
  ActiveOption()
  {
    activeState_ = ActiveState::STANDING;
  }

  void transition(const WalkManState& wmState);
  void action(WalkManState& wmState);

protected:
  enum class ActiveState
  {
    STANDING,
    WALKING,
    IN_WALK_KICK,
    NO_GROUND_CONTACT
  };
  /// the state of this suboption
  ActiveState activeState_;
};

void ActiveOption::transition(const WalkManState& wmState)
{
  // the active transitions
  switch (activeState_)
  {
    // for now fallback to standing
    case ActiveState::NO_GROUND_CONTACT:
      activeState_ = ActiveState::STANDING;
      break;
    case ActiveState::STANDING:
      if (wmState.motionPlannerOutput.bodyMotion == MotionPlannerOutput::BodyMotion::WALK &&
          wmState.motionActivation
                  .activations[static_cast<unsigned int>(MotionPlannerOutput::BodyMotion::WALK)] ==
              1.f &&
          !wmState.bodyPose.fallen && wmState.bodyPose.footContact)
      {
        if (wmState.motionPlannerOutput.walkData.inWalkKickType != InWalkKickType::NONE &&
            wmState.motionPlannerOutput.walkData.kickFoot != KickFoot::NONE)
        {
          wmState.walkGenerator.resetGenerator();
          activeState_ = ActiveState::IN_WALK_KICK;
        }
        else
        {
          // we are safe to start walking (not fallen, on ground and fully activated)
          wmState.walkGenerator.resetGenerator();
          activeState_ = ActiveState::WALKING;
        }
      }
      break;
    // for now fallback to walking
    case ActiveState::IN_WALK_KICK:
      if (!wmState.bodyPose.footContact || wmState.bodyPose.fallen)
      {
        // fell down or lost ground contact
        activeState_ = ActiveState::NO_GROUND_CONTACT;
      }
      else if ((!activeSubOption_ || activeSubOption_->actionComplete()) &&
               wmState.walkGenerator.t == 0)
      {
        if (wmState.motionPlannerOutput.bodyMotion == MotionPlannerOutput::BodyMotion::WALK)
        {
          activeState_ = ActiveState::WALKING;
        }
        else
        {
          activeState_ = ActiveState::STANDING;
        }
      }
      break;
    case ActiveState::WALKING:
      if (!wmState.bodyPose.footContact || wmState.bodyPose.fallen)
      {
        // fell down or lost ground contact
        activeState_ = ActiveState::NO_GROUND_CONTACT;
      }
      else if (wmState.motionPlannerOutput.bodyMotion != MotionPlannerOutput::BodyMotion::WALK &&
               wmState.walkGenerator.t == 0)
      {
        // were asked to stop walking -> standing
        activeState_ = ActiveState::STANDING;
      }
      else if (wmState.motionPlannerOutput.walkData.inWalkKickType != InWalkKickType::NONE &&
               wmState.motionPlannerOutput.walkData.kickFoot != KickFoot::NONE)
      {
        activeState_ = ActiveState::IN_WALK_KICK;
      }
      break;
  }
}

void ActiveOption::action(WalkManState& wmState)
{
  // the active actions
  switch (activeState_)
  {
    // for now fallback to standing
    case ActiveState::NO_GROUND_CONTACT:
      wmState.walkGenerator.resetGenerator();
      wmState.setWalkParametersForStand();
      break;
    case ActiveState::STANDING:
      wmState.setWalkParametersForStand();
      break;
    // for now fallback to walking
    case ActiveState::IN_WALK_KICK:
      callSubOption<InWalkKickOption>(wmState);
      break;
    case ActiveState::WALKING:
      wmState.lastTimeWalking = wmState.cycleInfo.startTime;
      callSubOption<WalkingOption>(wmState);
      break;
  }
}
