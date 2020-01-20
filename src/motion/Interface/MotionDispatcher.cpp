#include "MotionDispatcher.hpp"


MotionDispatcher::MotionDispatcher(const ModuleManagerInterface& manager)
  : Module(manager)
  , bodyPose_(*this)
  , cycleInfo_(*this)
  , fallManagerOutput_(*this)
  , jumpOutput_(*this)
  , kickOutput_(*this)
  , poserOutput_(*this)
  , sitDownOutput_(*this)
  , sitUpOutput_(*this)
  , standUpOutput_(*this)
  , walkingEngineWalkOutput_(*this)
  , motionRequest_(*this)
  , motionActivation_(*this)
  , lastActiveMotion_(MotionRequest::BodyMotion::DEAD)
  , headMotionActivation_(0.f)
  , fallManagerActive_(false)
  , timeWhenFallManagerFinished_()
{
  activations_.fill(0.f);
  activations_[static_cast<unsigned int>(MotionRequest::BodyMotion::DEAD)] = 1;
}

void MotionDispatcher::cycle()
{
  /* If fallen all motion requests are discarded until the fall manager is finished.
   * Once the fall manager is finished the angles are held until a stand up motion request is
   * received. In any case there is at least 1 s between end of the fall manager and the
   * initialization of stand up.
   */

  if (fallManagerOutput_->wantToSend)
  {
    // we are currently falling. The fallManager's output should be applied to the joints.
    fallManagerActive_ = true;
    motionActivation_->activeMotion = MotionRequest::BodyMotion::FALL_MANAGER;
  }
  else if (bodyPose_->fallen && fallManagerActive_)
  {
    // we started to notice that we are fallen but the fallManager is still active. Save the current
    // time so we can wait one moment before we start standing up.
    fallManagerActive_ = false;
    timeWhenFallManagerFinished_ = cycleInfo_->startTime;
  }
  else if (lastActiveMotion_ == MotionRequest::BodyMotion::SIT_DOWN)
  {
    if (sitDownOutput_->safeExit)
    {
      if (motionRequest_->bodyMotion != MotionRequest::BodyMotion::SIT_DOWN)
      {
        motionActivation_->activeMotion = MotionRequest::BodyMotion::SIT_UP;
      }
    }
  }
  else if ((lastActiveMotion_ == MotionRequest::BodyMotion::FALL_MANAGER &&
            cycleInfo_->getTimeDiff(timeWhenFallManagerFinished_) > 1.f) ||
           lastActiveMotion_ == MotionRequest::BodyMotion::DEAD ||
           lastActiveMotion_ == MotionRequest::BodyMotion::STAND ||
           (lastActiveMotion_ == MotionRequest::BodyMotion::WALK &&
            walkingEngineWalkOutput_->safeExit) ||
           (lastActiveMotion_ == MotionRequest::BodyMotion::KICK && kickOutput_->safeExit) ||
           lastActiveMotion_ == MotionRequest::BodyMotion::PENALIZED ||
           (lastActiveMotion_ == MotionRequest::BodyMotion::JUMP && jumpOutput_->safeExit) ||
           (lastActiveMotion_ == MotionRequest::BodyMotion::STAND_UP && standUpOutput_->safeExit) ||
           lastActiveMotion_ == MotionRequest::BodyMotion::HOLD ||
           (lastActiveMotion_ == MotionRequest::BodyMotion::SIT_UP && sitUpOutput_->safeExit))
  {
    // It is safe to start a new motion.
    motionActivation_->startInterpolation = true;
    motionActivation_->activeMotion = motionRequest_->bodyMotion;
  }

  lastActiveMotion_ = motionActivation_->activeMotion;
  float sum = 0;
  float delta = 0.01;
  if (lastActiveMotion_ == MotionRequest::BodyMotion::FALL_MANAGER ||
      lastActiveMotion_ == MotionRequest::BodyMotion::STAND_UP ||
      lastActiveMotion_ == MotionRequest::BodyMotion::KICK ||
      lastActiveMotion_ == MotionRequest::BodyMotion::JUMP ||
      lastActiveMotion_ == MotionRequest::BodyMotion::SIT_UP)
  {
    delta = 1;
  }
  for (unsigned int i = 0; i < activations_.size(); i++)
  {
    if (i == static_cast<unsigned int>(lastActiveMotion_))
    {
      activations_[i] += delta;
    }
    else
    {
      activations_[i] -= delta;
    }
    if (activations_[i] < 0)
    {
      activations_[i] = 0;
    }
    sum += activations_[i];
  }
  if (sum != 0)
  {
    for (unsigned int i = 0; i < activations_.size(); i++)
    {
      activations_[i] /= static_cast<double>(sum);
    }
  }

  // Handle the head separately
  if (motionRequest_->headMotion != MotionRequest::HeadMotion::BODY && !bodyPose_->fallen)
  {
    headMotionActivation_ = std::min(1.f, headMotionActivation_ + delta);
  }
  else
  {
    headMotionActivation_ = std::max(0.f, headMotionActivation_ - delta);
  }
  motionActivation_->activations = activations_;
  motionActivation_->headMotionActivation = headMotionActivation_;
  motionActivation_->headCanBeUsed = !motionRequest_->usesHead();
  motionActivation_->armsCanBeUsed = !motionRequest_->usesArms();
}
