#pragma once

#include "Framework/Module.hpp"

#include "Data/ActionCommand.hpp"
#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/KickConfigurationData.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/StepPlan.hpp"
#include "Data/WalkGeneratorOutput.hpp"
#include "Data/WalkManagerOutput.hpp"

class Motion;

/**
 * @brief Walkmanager implements the walking state machine, while considering multiple walk modes
 * The walk manager constructs the stateful requests for the WalkGenerator.
 * All state is kept in the members rootState_ and activeState_ and InWalkKickState.
 * The RootState toggles between being active, which means the WalkGenerator has to generate
 * something (i.e. Brain wants to walk or stand). The ActiveState describes the different states
 * in which walking can be. The transition functions model all transitions between the respective
 * states (i.e. manipulate rootState_/activeState_/inWalkKickState_) and is called every cycle.
 * Based on the state information, the WalkManager constructs the requests to pass to the
 * WalkGenerator.
 */
class WalkManager : public Module<WalkManager, Motion>
{
public:
  ModuleName name__{"WalkManager"};

  explicit WalkManager(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  const Dependency<ActionCommand> actionCommand_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<KickConfigurationData> kickConfigurationData_;
  /// a reference to the motion activation of last cycle
  const Reference<MotionActivation> motionActivation_;
  const Dependency<StepPlan> stepPlan_;

  Production<WalkManagerOutput> walkManagerOutput_;

  enum class RootState
  {
    ACTIVE,
    INACTIVE
  };
  enum class ActiveState
  {
    STANDING,
    WALKING,
    IN_WALK_KICK,
    NO_GROUND_CONTACT
  };
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
  /// the state of the root option
  RootState rootState_ = RootState::INACTIVE;
  /// the state of the active option
  ActiveState activeState_ = ActiveState::STANDING;
  /// the state of the in walk kick option
  InWalkKickState inWalkKickState_ = InWalkKickState::INIT;
  /// a backup of the requested in walk kick for a stable decision during in walk kicks
  // InWalkKickType inWalkKickTypeBackup_ = InWalkKickType::NONE;
  /// a backup of the requested in walk kick for a stable decision during in walk kicks
  // KickFoot kickFootBackup_ = KickFoot::NONE;
  /// the last target processed
  Clock::time_point lastTimeWalking_;

  /**
   * @brief Transition between the states of the RootState. Takes the current state and returns the
   * new state based on met conditions.
   * @param currentState the state of the RootState to transition from
   * @return the reached state after transitioning
   */
  RootState transition(RootState currentState) const;
  /**
   * @brief Transition between the states of the ActiveState. Takes the current state and returns
   * the new state based on met conditions.
   * @param currentState the state of the ActiveState to transition from
   * @return the reached state after transitioning
   */
  ActiveState transition(ActiveState currentState) const;
  /**
   * @brief Transition between the states of the InWalkKickState. Takes the current state and
   * returns the new state based on met conditions.
   * @param currentState the state of the InWalkKickState to transition from
   * @return the reached state after transitioning
   */
  InWalkKickState transition(InWalkKickState currentState);
};
