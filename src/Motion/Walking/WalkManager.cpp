#include "Motion/Walking/WalkManager.hpp"

WalkManager::WalkManager(const ModuleManagerInterface& manager)
  : Module{manager}
  , actionCommand_{*this}
  , bodyPose_{*this}
  , cycleInfo_{*this}
  , kickConfigurationData_{*this}
  , motionActivation_{*this}
  , stepPlan_{*this}
  , walkManagerOutput_{*this}
{
}

void WalkManager::cycle()
{
  // make transitions of the root state
  rootState_ = transition(rootState_);

  if (rootState_ == RootState::INACTIVE)
  {
    walkManagerOutput_->isActive = false;
    walkManagerOutput_->valid = true;
    return;
  }
  // the walking is active now
  walkManagerOutput_->isActive = true;
  walkManagerOutput_->action = WalkManagerOutput::RequestAction::WALK;
  float forward = 0.f;
  float left = 0.f;
  float turn = 0.f;
  // the kickFootOffset is not used
  auto emptyKickFootOffset = std::function<KinematicMatrix(float)>();

  // handle state transition of the active state
  activeState_ = transition(activeState_);
  switch (activeState_)
  {
    case ActiveState::NO_GROUND_CONTACT:
      walkManagerOutput_->action = WalkManagerOutput::RequestAction::RESET;
      break;
    case ActiveState::STANDING: {
      walkManagerOutput_->action = WalkManagerOutput::RequestAction::STAND;
      break;
    }
    case ActiveState::IN_WALK_KICK: {
      // TODO
      forward = 0.f;
      left = 0.f;
      turn = 0.f;
      break;
    }
    case ActiveState::WALKING: {
      forward = stepPlan_->forward;
      left = stepPlan_->left;
      turn = stepPlan_->turn;
    }
    break;
  }
  walkManagerOutput_->forward = forward;
  walkManagerOutput_->left = left;
  walkManagerOutput_->turn = turn;
  walkManagerOutput_->valid = true;
}

WalkManager::RootState WalkManager::transition(const RootState currentState) const
{
  const bool activated =
      (motionActivation_->activations[ActionCommand::Body::MotionType::WALK] > 0.f) ||
      (motionActivation_->activations[ActionCommand::Body::MotionType::STAND] > 0.f);
  // the root transitions
  switch (currentState)
  {
    case RootState::INACTIVE:
      if (activated)
      {
        // we are now active
        return RootState::ACTIVE;
      }
      return RootState::INACTIVE;
    case RootState::ACTIVE:
      if (!activated)
      {
        return RootState::INACTIVE;
      }
      return RootState::ACTIVE;
  }
  // keep the current state
  return currentState;
}

WalkManager::ActiveState WalkManager::transition(const ActiveState currentState) const
{
  // the active transitions
  switch (currentState)
  {
    // for now fallback to standing
    case ActiveState::NO_GROUND_CONTACT:
      return ActiveState::STANDING;
    case ActiveState::STANDING:
      if (actionCommand_->body().type != ActionCommand::Body::MotionType::WALK ||
          motionActivation_->activations[ActionCommand::Body::MotionType::WALK] != 1.f ||
          bodyPose_->fallen || !bodyPose_->footContact)
      {
        // it is not safe to start walking here
        return ActiveState::STANDING;
      }
      if (actionCommand_->body().inWalkKickType != InWalkKickType::NONE)
      {
        assert(actionCommand_->body().kickFoot != KickFoot::NONE);
        return ActiveState::IN_WALK_KICK;
      }
      // we are safe to start walking
      return ActiveState::WALKING;
    case ActiveState::IN_WALK_KICK:
      if (!bodyPose_->footContact || bodyPose_->fallen)
      {
        // fell down or lost ground contact
        return ActiveState::NO_GROUND_CONTACT;
      }
      if (inWalkKickState_ == InWalkKickState::KICK /* TODO && walkGeneratorOutput_->t == 0*/)
      {
        if (actionCommand_->body().type == ActionCommand::Body::MotionType::WALK)
        {
          return ActiveState::WALKING;
        }
        return ActiveState::STANDING;
      }
      return ActiveState::IN_WALK_KICK;
    case ActiveState::WALKING:
      if (!bodyPose_->footContact || bodyPose_->fallen)
      {
        // fell down or lost ground contact
        return ActiveState::NO_GROUND_CONTACT;
      }
      else if (actionCommand_->body().type != ActionCommand::Body::MotionType::WALK)
      {
        // were asked to stop walking -> standing
        return ActiveState::STANDING;
      }
      else if (actionCommand_->body().inWalkKickType != InWalkKickType::NONE)
      {
        assert(actionCommand_->body().kickFoot != KickFoot::NONE);
        return ActiveState::IN_WALK_KICK;
      }
      return ActiveState::WALKING;
  }
  // keep the current state
  return currentState;
}

WalkManager::InWalkKickState WalkManager::transition(const InWalkKickState currentState)
{
  // TODO revisit after china
  /*
  switch (currentState)
  {
    case InWalkKickState::INIT:
      inWalkKickTypeBackup_ = actionCommand_->body().inWalkKickType;
      kickFootBackup_ = actionCommand_->body().kickFoot;
      [[fallthrough]];
    case InWalkKickState::WAIT: {
      if (walkGeneratorOutput_->t != 0)
      {
        // we can not start yet and have to wait for the correct foot
        // this is necessary when falling through from the top
        return InWalkKickState::WAIT;
      }

      // the foot used for the pre step is the one that is not kicking
      const bool leftPrestepPhase = kickFootBackup_ == KickFoot::RIGHT;
      // load the kick from the kick provider
      const InWalkKick& inWalkKick = kickConfigurationData_->inWalkKicks[inWalkKickTypeBackup_];

      // we can only start at the beginning of the new step.
      // did a step phase of the correct foot just start?
      if (inWalkKick.requiresPrestep && walkGeneratorOutput_->isLeftPhase == leftPrestepPhase)
      {
        return InWalkKickState::PRE_STEP;
      }
      if (!inWalkKick.requiresPrestep && walkGeneratorOutput_->isLeftPhase != leftPrestepPhase)
      {
        return InWalkKickState::START;
      }
      return InWalkKickState::WAIT;
    }
    case InWalkKickState::PRE_STEP:
      if (walkGeneratorOutput_->t == 0)
      {
        // we are at the beginning of the next step. Thus the prestep is finished.
        return InWalkKickState::START;
      }
      return InWalkKickState::PRE_STEP;
    case InWalkKickState::START:
      return InWalkKickState::KICK;
    case InWalkKickState::KICK:
      // this is the target state. There is no way to get out of here.
      return InWalkKickState::KICK;
  }
  */
  // keep the current state
  return currentState;
}
