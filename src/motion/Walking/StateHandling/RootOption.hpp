#pragma once

#include "ActiveOption.hpp"
#include "Data/WalkingEngineWalkOutput.hpp"
#include "Modules/NaoProvider.h"
#include "Modules/Poses.h"
#include "WalkManState.hpp"
#include "WalkOptionInterface.hpp"

/**
 * @brief RootOption the root of the option tree. Handles transitions between active and incative.
 * Will call the active option if active. Otherwise, the output is set to some default angles.
 */
class RootOption : public WalkOptionInterface<WalkingEngineWalkOutput&>
{
public:
  RootOption()
  {
    rootState_ = RootState::INACTIVE;
  }

  WalkingEngineWalkOutput& run(WalkManState& wmState)
  {
    transition(wmState);
    return action(wmState);
  }

  void transition(const WalkManState& wmState);
  WalkingEngineWalkOutput& action(WalkManState& wmState);

protected:
  enum class RootState
  {
    ACTIVE,
    INACTIVE
  };
  /// the state of this option
  RootState rootState_;
  /// the final calculated output
  WalkingEngineWalkOutput output_;
};

void RootOption::transition(const WalkManState& wmState)
{
  // the root transitions
  switch (rootState_)
  {
    case RootState::INACTIVE:
      if ((wmState.motionActivation
               .activations[static_cast<unsigned int>(MotionPlannerOutput::BodyMotion::WALK)] >
           0.f) ||
          (wmState.motionActivation
               .activations[static_cast<unsigned int>(MotionPlannerOutput::BodyMotion::STAND)] >
           0.f))
      {
        // we are now active
        rootState_ = RootState::ACTIVE;
      }
      break;
    case RootState::ACTIVE:
      if ((wmState.motionActivation
               .activations[static_cast<unsigned int>(MotionPlannerOutput::BodyMotion::WALK)] ==
           0.f) &&
          (wmState.motionActivation
               .activations[static_cast<unsigned int>(MotionPlannerOutput::BodyMotion::STAND)] ==
           0.f))
      {
        rootState_ = RootState::INACTIVE;
      }
      break;
  }
}

WalkingEngineWalkOutput& RootOption::action(WalkManState& wmState)
{
  // the root actions
  switch (rootState_)
  {
    case RootState::INACTIVE:
      // neutral element of the walk output
      output_.angles = Poses::getPose(Poses::READY);
      output_.stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 1.f);
      output_.stepOffset = Pose(0.f, 0.f, 0.f);
      output_.safeExit = true;
      // reset the generator
      wmState.walkGenerator.resetGenerator();
      break;
    case RootState::ACTIVE:
      // run the active option. Handles/Resolves the active state in greater detail.
      // This will manipulate speed, target, walkMode and getKickFootOffset of the wmState
      callSubOption<ActiveOption>(wmState);
      // call the generator to calculate the joints
      wmState.walkGenerator.calcJoints(wmState.speed, wmState.target, wmState.walkPathGradient,
                                       wmState.walkMode, wmState.getKickFootOffset);
      // write the joints to the output_
      output_.safeExit = wmState.cycleInfo.getTimeDiff(wmState.lastTimeWalking) >=
                             wmState.minTimeInStandBeforeLeaving &&
                         wmState.walkGenerator.armState == WalkGenerator::ArmState::NORMAL;
      output_.angles = wmState.walkGenerator.angles;
      output_.stiffnesses = wmState.walkGenerator.stiffnesses;
      output_.stepOffset = wmState.walkGenerator.odometryOffset;
      break;
  }


  // set the velocity components used in brain to estimate the time to reach a certain pose
  output_.maxVelocityComponents = wmState.walkGenerator.maxSpeed;
  output_.walkAroundBallVelocity = wmState.walkGenerator.maxSpeed.orientation * 0.5f;

  return output_;
}
