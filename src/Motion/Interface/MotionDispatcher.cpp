#include "Motion/Interface/MotionDispatcher.hpp"

MotionDispatcher::MotionDispatcher(const ModuleManagerInterface& manager)
  : Module(manager)
  , actionCommand_(*this)
  , bodyPose_(*this)
  , cycleInfo_(*this)
  , fallManagerOutput_(*this)
  , jumpOutput_(*this)
  , kickOutput_(*this)
  , sitDownOutput_(*this)
  , sitUpOutput_(*this)
  , standUpOutput_(*this)
  , walkGeneratorOutput_(*this)
  , motionActivation_(*this)
  , lastActiveMotion_(ActionCommand::Body::MotionType::DEAD)
  , timeWhenFallManagerFinished_(cycleInfo_->startTime)
{
  activations_.fill(0.f);
  activations_[ActionCommand::Body::MotionType::DEAD] = 1.f;
}

void MotionDispatcher::cycle()
{
  using BodyMotion = ActionCommand::Body::MotionType;
  using HeadMotion = ActionCommand::Head::MotionType;
  /* If DEAD is requested it takes precedence over all other motion outputs. If the robot is held in
   * the air (no foot contact) it will transition to DEAD immediately (transition = output this
   * motion). When standing on the ground (foot contact) we transition first to SIT_DOWN, wait for
   * the motion to finish, and then transition to DEAD (to sit down safely).
   */
  if (actionCommand_->body().type == ActionCommand::Body::MotionType::DEAD)
  {
    if (!bodyPose_->footContact ||
        (lastActiveMotion_ == ActionCommand::Body::MotionType::SIT_DOWN &&
         sitDownOutput_->safeExit))
    {
      motionActivation_->activeMotion = ActionCommand::Body::MotionType::DEAD;
    }
    else if (lastActiveMotion_ != ActionCommand::Body::MotionType::DEAD)
    {
      motionActivation_->activeMotion = ActionCommand::Body::MotionType::SIT_DOWN;
    }
    else
    {
      // either DEAD or SIT_DOWN
      motionActivation_->activeMotion = lastActiveMotion_;
    }
  }
  /* If fallen all motion requests are discarded until the fall manager is finished.
   * Once the fall manager is finished the angles are held until a stand up motion request is
   * received. In any case there is at least 1 s between end of the fall manager and the
   * initialization of stand up.
   */
  else if (fallManagerOutput_->wantToSend)
  {
    // we are currently falling. The fallManager's output should be applied to the joints.
    fallManagerActive_ = true;
    motionActivation_->activeMotion = ActionCommand::Body::MotionType::FALL_MANAGER;
  }
  else if (bodyPose_->fallen && fallManagerActive_)
  {
    // we started to notice that we are fallen and the fall manager does not want to send anymore,
    // but the fallManager was active. Save the current time so we can wait one moment before
    // we start standing up.
    fallManagerActive_ = false;
    timeWhenFallManagerFinished_ = cycleInfo_->startTime;
  }
  else if (lastActiveMotion_ == BodyMotion::SIT_DOWN && sitDownOutput_->safeExit &&
           actionCommand_->body().type != BodyMotion::SIT_DOWN)
  {
    motionActivation_->activeMotion = BodyMotion::SIT_UP;
  }
  else if ((lastActiveMotion_ == BodyMotion::FALL_MANAGER &&
            cycleInfo_->getAbsoluteTimeDifference(timeWhenFallManagerFinished_) > 1s) ||
           lastActiveMotion_ == BodyMotion::DEAD || lastActiveMotion_ == BodyMotion::STAND ||
           lastActiveMotion_ == BodyMotion::PUPPET ||
           (lastActiveMotion_ == BodyMotion::WALK && walkGeneratorOutput_->safeExit) ||
           (lastActiveMotion_ == BodyMotion::KICK && kickOutput_->safeExit) ||
           lastActiveMotion_ == BodyMotion::PENALIZED ||
           (lastActiveMotion_ == BodyMotion::JUMP && jumpOutput_->safeExit) ||
           (lastActiveMotion_ == BodyMotion::STAND_UP && standUpOutput_->safeExit) ||
           lastActiveMotion_ == BodyMotion::HOLD ||
           (lastActiveMotion_ == BodyMotion::SIT_UP && sitUpOutput_->safeExit))
  {
    motionActivation_->activeMotion = actionCommand_->body().type;
  }
  else
  {
    motionActivation_->activeMotion = lastActiveMotion_;
  }

  float delta = 0.01f;
  if (motionActivation_->activeMotion == BodyMotion::FALL_MANAGER ||
      motionActivation_->activeMotion == BodyMotion::KICK ||
      motionActivation_->activeMotion == BodyMotion::JUMP)
  {
    // instantly apply these motion types, do not care what was before
    delta = 1.f;
  }

  float sum = 0.f;
  for (std::size_t i = 0; i < activations_.size(); i++)
  {
    const auto motionType = static_cast<ActionCommand::Body::MotionType>(i);
    if (motionType == motionActivation_->activeMotion)
    {
      activations_[motionType] += delta;
    }
    else
    {
      activations_[motionType] -= delta;
    }
    activations_[motionType] = std::clamp(activations_[motionType], 0.f, 1.f);
    sum += activations_[motionType];
  }
  if (sum != 0)
  {
    for (auto& activation : activations_)
    {
      activation /= static_cast<double>(sum);
    }
  }

  // Handle the head separately
  if (!actionCommand_->body().usesHead() && actionCommand_->head().type != HeadMotion::BODY &&
      !bodyPose_->fallen)
  {
    headMotionActivation_ = std::clamp(headMotionActivation_ + delta, 0.f, 1.f);
  }
  else
  {
    headMotionActivation_ = std::clamp(headMotionActivation_ - delta, 0.f, 1.f);
  }
  motionActivation_->activations = activations_;
  motionActivation_->headMotionActivation = headMotionActivation_;
  motionActivation_->headCanBeUsed = !actionCommand_->body().usesHead();
  motionActivation_->armsCanBeUsed = !actionCommand_->body().usesArms();
  // store state for next cycle
  lastActiveMotion_ = motionActivation_->activeMotion;
}
