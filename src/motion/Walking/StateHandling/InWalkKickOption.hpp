#pragma once

#include "Tools/Kinematics/KinematicMatrix.h"
#include "WalkManState.hpp"
#include "WalkOptionInterface.hpp"
#include "WalkingOption.hpp"

/**
 * @brief InWalkKickOption handles the details of state transitions during an in walk kick.
 */
class InWalkKickOption : public WalkOptionInterface<void>
{
public:
  InWalkKickOption()
  {
    inWalkKickState_ = InWalkKickState::INIT;
  }

  virtual bool actionComplete()
  {
    return inWalkKickState_ == InWalkKickState::KICK;
  }

  void transition(const WalkManState& wmState);
  void action(WalkManState& wmState);

protected:
  enum class InWalkKickState
  {
    /// setup parameters for kicking
    INIT,
    /// wait until the right timing is reached
    WAIT,
    /// perform a pre step that prepares the actual kick (e.g. step next to the ball)
    PRE_STEP,
    /// perform the actual kicking motion
    START,
    /// actually kicking
    KICK
  };
  /// the state of this suboption
  InWalkKickState inWalkKickState_;
  /// a backup of the requested walk data (from the motion planner output) for a stable decision
  /// during in walk kicks
  WalkData kickBackupWalkData = WalkData();
};

void InWalkKickOption::transition(const WalkManState& wmState)
{
  switch (inWalkKickState_)
  {
    case InWalkKickState::INIT:
      kickBackupWalkData = wmState.motionPlannerOutput.walkData;
      /*FALLTHROUGH*/
      [[fallthrough]];
    case InWalkKickState::WAIT:
      if (wmState.walkGenerator.t == 0)
      {
        // the foot used for the pre step is the one that is not kicking
        const bool leftPrestepPhase = kickBackupWalkData.kickFoot == KickFoot::RIGHT;
        // load the kick from the kick provider
        const InWalkKick& inWalkKick =
            wmState.kickConfigurationData
                .inWalkKicks[static_cast<int>(kickBackupWalkData.inWalkKickType)];

        // we can only start at the beginning of the new step.
        // did a step phase of the correct foot just start?
        if (inWalkKick.requiresPrestep && wmState.walkGenerator.isLeftPhase == leftPrestepPhase)
        {
          inWalkKickState_ = InWalkKickState::PRE_STEP;
          return;
        }
        else if (!inWalkKick.requiresPrestep &&
                 wmState.walkGenerator.isLeftPhase != leftPrestepPhase)
        {
          inWalkKickState_ = InWalkKickState::START;
          return;
        }
      }
      // we can not start yet and have to wait for the correct foot
      // this is necessary when falling through from the top
      inWalkKickState_ = InWalkKickState::WAIT;
      break;
    case InWalkKickState::PRE_STEP:
      if (wmState.walkGenerator.t == 0)
      {
        // we are at the beginning of the next step. Thus the prestep is finished.
        inWalkKickState_ = InWalkKickState::START;
      }
      break;
    case InWalkKickState::START:
      inWalkKickState_ = InWalkKickState::KICK;
      break;
    case InWalkKickState::KICK:
      // this is the target state. There is no way to get out of here.
      break;
  }
}

void InWalkKickOption::action(WalkManState& wmState)
{
  const float stepSign = kickBackupWalkData.kickFoot == KickFoot::LEFT ? 1.f : -1.f;

  const InWalkKick& inWalkKick =
      wmState.kickConfigurationData
          .inWalkKicks[static_cast<int>(kickBackupWalkData.inWalkKickType)];

  switch (inWalkKickState_)
  {
    case InWalkKickState::INIT:
      assert(false && "How did we get here? Transition failed!");
      break;
    case InWalkKickState::WAIT:
      // we can not do anything yet and have to keep walking until the correct foot is free
      wmState.setWalkParametersForStepSizeMode(Pose());
      break;
    case InWalkKickState::PRE_STEP:
    {
      // do a prestep in step size mode without any kick trajectory
      wmState.setWalkParametersForStepSizeMode(Pose(inWalkKick.preStep.position.x(),
                                                    stepSign * inWalkKick.preStep.position.y(),
                                                    stepSign * inWalkKick.preStep.orientation));
      break;
    }
    case InWalkKickState::START:
    {
      // do a prestep in step size mode without any kick trajectory
      wmState.setWalkParametersForStepSizeMode(Pose(inWalkKick.kickStep.position.x(),
                                                    stepSign * inWalkKick.kickStep.position.y(),
                                                    stepSign * inWalkKick.kickStep.orientation));
      break;
    }
    case InWalkKickState::KICK:
    {
      auto getKickFootOffset = std::function<KinematicMatrix(float)>();
      // do a prestep in step size mode without any kick trajectory
      wmState.setWalkParametersForStepSizeMode(Pose(inWalkKick.kickStep.position.x(),
                                                    stepSign * inWalkKick.kickStep.position.y(),
                                                    stepSign * inWalkKick.kickStep.orientation),
                                               getKickFootOffset);
    }
  }
}
