#include "MotionDispatcher.hpp"


MotionDispatcher::MotionDispatcher(const ModuleManagerInterface& manager)
  : Module(manager, "MotionDispatcher")
  , keeperOutput_(*this)
  , kickOutput_(*this)
  , poserOutput_(*this)
  , standUpOutput_(*this)
  , walkingEngineWalkOutput_(*this)
  , motionRequest_(*this)
  , motionActivation_(*this)
  , lastActiveMotion_(MotionRequest::BodyMotion::DEAD)
  , headMotionActivation_(0.f)
{
  activations_.fill(0.f);
  activations_[static_cast<unsigned int>(MotionRequest::BodyMotion::DEAD)] = 1;
}

void MotionDispatcher::cycle()
{
  if (lastActiveMotion_ == MotionRequest::BodyMotion::DEAD                                            //
      || lastActiveMotion_ == MotionRequest::BodyMotion::STAND                                        //
      || (lastActiveMotion_ == MotionRequest::BodyMotion::WALK && walkingEngineWalkOutput_->safeExit) //
      || (lastActiveMotion_ == MotionRequest::BodyMotion::KICK && kickOutput_->safeExit)              //
      || lastActiveMotion_ == MotionRequest::BodyMotion::PENALIZED                                    //
      || (lastActiveMotion_ == MotionRequest::BodyMotion::KEEPER && keeperOutput_->safeExit)          //
      || (lastActiveMotion_ == MotionRequest::BodyMotion::STAND_UP && standUpOutput_->safeExit)       //
      || lastActiveMotion_ == MotionRequest::BodyMotion::HOLD)
  {
    motionActivation_->startInterpolation = true;
    motionActivation_->activeMotion = motionRequest_->bodyMotion;
  }
  else
  {
    motionActivation_->activeMotion = lastActiveMotion_;
  }
  lastActiveMotion_ = motionActivation_->activeMotion;
  float sum = 0;
  float delta = 0.01;
  if (lastActiveMotion_ == MotionRequest::BodyMotion::STAND_UP //
      || lastActiveMotion_ == MotionRequest::BodyMotion::KICK  //
      || lastActiveMotion_ == MotionRequest::BodyMotion::KEEPER)
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

  // Handle the head seperately
  if (motionRequest_->headMotion != MotionRequest::HeadMotion::BODY)
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
