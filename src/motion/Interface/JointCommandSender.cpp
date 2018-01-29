#include "Modules/NaoProvider.h"

#include "JointCommandSender.hpp"

JointCommandSender::JointCommandSender(const ModuleManagerInterface& manager)
  : Module(manager, "JointCommandSender")
  , motionRequest_(*this)
  , motionActivation_(*this)
  , fallManagerOutput_(*this)
  , headMotionOutput_(*this)
  , keeperOutput_(*this)
  , kickOutput_(*this)
  , pointOutput_(*this)
  , poserOutput_(*this)
  , standUpOutput_(*this)
  , walkingEngineWalkOutput_(*this)
  , walkingEngineStandOutput_(*this)
  , jointSensorData_(*this)
  , motionState_(*this)
  , angles_(JOINTS::JOINTS_MAX, 0.f)
  , stiffnesses_(JOINTS::JOINTS_MAX, -1.f)
{
}

void JointCommandSender::cycle()
{
  if (motionActivation_->startInterpolation)
  {
    startInterpolationAngles_ = jointSensorData_->getBodyAngles();
  }
  if (motionRequest_->bodyMotion == MotionRequest::BodyMotion::DEAD)
  {
    // This handles the double chest button press which should always take priority.
    for (unsigned int i = 0; i < JOINTS::JOINTS_MAX; i++)
    {
      stiffnesses_[i] = -1.f;
    }
    motionState_->bodyMotion = MotionRequest::BodyMotion::DEAD;
    motionState_->leftArmMotion = MotionRequest::ArmMotion::BODY;
    motionState_->rightArmMotion = MotionRequest::ArmMotion::BODY;
    motionState_->headMotion = MotionRequest::HeadMotion::BODY;
  }
  else if (fallManagerOutput_->wantToSend)
  {
    for (unsigned int i = 0; i < JOINTS::JOINTS_MAX; i++)
    {
      angles_[i] = fallManagerOutput_->angles[i];
      stiffnesses_[i] = fallManagerOutput_->stiffnesses[i];
    }
    // HOLD is a dummy value because the fall manager cannot be requested.
    motionState_->bodyMotion = MotionRequest::BodyMotion::HOLD;
    motionState_->leftArmMotion = MotionRequest::ArmMotion::BODY;
    motionState_->rightArmMotion = MotionRequest::ArmMotion::BODY;
    motionState_->headMotion = MotionRequest::HeadMotion::BODY;
  }
  else if (motionRequest_->bodyMotion == MotionRequest::BodyMotion::HOLD)
  {
    // keep the angles from the previous cycle
    for (unsigned int i = 0; i < JOINTS::JOINTS_MAX; i++)
    {
      stiffnesses_[i] = 1.f;
    }
    motionState_->bodyMotion = MotionRequest::BodyMotion::HOLD;
    motionState_->leftArmMotion = MotionRequest::ArmMotion::BODY;
    motionState_->rightArmMotion = MotionRequest::ArmMotion::BODY;
    motionState_->headMotion = MotionRequest::HeadMotion::BODY;
  }
  else
  {
    // This sum can be < 1 when dead or hold are active.
    float sum = motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::KEEPER)] +
                motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::KICK)] +
                motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::STAND_UP)] +
                motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::PENALIZED)] +
                motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::WALK)] +
                motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::STAND)];
    for (unsigned int i = 0; i < JOINTS::JOINTS_MAX; i++)
    {
      angles_[i] = motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::KEEPER)] * keeperOutput_->angles[i] +
                   motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::KICK)] * kickOutput_->angles[i] +
                   motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::STAND_UP)] * standUpOutput_->angles[i] +
                   motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::PENALIZED)] * poserOutput_->angles[i] +
                   motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::WALK)] * walkingEngineWalkOutput_->angles[i] +
                   motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::STAND)] * walkingEngineStandOutput_->angles[i] +
                   (1 - sum) * startInterpolationAngles_[i]; // This is needed for interpolating from dead or hold.
      float stiffness = 0;
      // This gets the highest stiffness of all activated motions.
      if (motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::KEEPER)] > 0 && keeperOutput_->stiffnesses[i] > stiffness)
      {
        stiffness = keeperOutput_->stiffnesses[i];
      }
      if (motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::KICK)] > 0 && kickOutput_->stiffnesses[i] > stiffness)
      {
        stiffness = kickOutput_->stiffnesses[i];
      }
      if (motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::STAND_UP)] > 0 && standUpOutput_->stiffnesses[i] > stiffness)
      {
        stiffness = standUpOutput_->stiffnesses[i];
      }
      if (motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::PENALIZED)] > 0 && poserOutput_->stiffnesses[i] > stiffness)
      {
        stiffness = poserOutput_->stiffnesses[i];
      }
      if (motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::WALK)] > 0 &&
          walkingEngineWalkOutput_->stiffnesses[i] > stiffness)
      {
        stiffness = walkingEngineWalkOutput_->stiffnesses[i];
      }
      if (motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::STAND)] > 0 &&
          walkingEngineStandOutput_->stiffnesses[i] > stiffness)
      {
        stiffness = walkingEngineStandOutput_->stiffnesses[i];
      }
      stiffnesses_[i] = stiffness;
    }
    motionState_->bodyMotion = motionActivation_->activeMotion;
    motionState_->leftArmMotion = MotionRequest::ArmMotion::BODY;
    motionState_->rightArmMotion = MotionRequest::ArmMotion::BODY;
    motionState_->headMotion = MotionRequest::HeadMotion::BODY;
  }
  // The head motion can be trusted that it only wants to send when it is allowed to.
  if (motionActivation_->headMotionActivation > 0.f)
  {
    for (unsigned int i = 0; i < JOINTS_HEAD::HEAD_MAX; i++)
    {
      angles_[JOINTS::HEAD_YAW + i] = (1.f - motionActivation_->headMotionActivation) * angles_[JOINTS::HEAD_YAW + i] +
                                      motionActivation_->headMotionActivation * headMotionOutput_->angles[i];
      stiffnesses_[JOINTS::HEAD_YAW + i] = 0.9f;
    }
    motionState_->headMotion = MotionRequest::HeadMotion::ANGLES;
  }
  // The pointer can be trusted, too. Well, actually not.
  if (pointOutput_->wantToSend)
  {
    for (unsigned int i = 0; i < JOINTS_L_ARM::L_ARM_MAX; i++)
    {
      angles_[JOINTS::L_SHOULDER_PITCH + i] = pointOutput_->leftAngles[i];
      angles_[JOINTS::R_SHOULDER_PITCH + i] = pointOutput_->rightAngles[i];
      stiffnesses_[JOINTS::L_SHOULDER_PITCH + i] = pointOutput_->stiffnesses[i];
      stiffnesses_[JOINTS::R_SHOULDER_PITCH + i] = pointOutput_->stiffnesses[JOINTS_L_ARM::L_ARM_MAX + i];
    }
    motionState_->leftArmMotion = MotionRequest::ArmMotion::POINT;
    motionState_->rightArmMotion = MotionRequest::ArmMotion::POINT;
  }
#ifndef NDEBUG
  for (unsigned int i = 0; i < JOINTS::JOINTS_MAX; i++)
  {
    if (std::isnan(kickOutput_->angles[i]))
    {
      std::cout << "KickOutput " << i << " was NaN" << '\n';
    }
    if (std::isnan(walkingEngineWalkOutput_->angles[i]))
    {
      std::cout << "WalkingEngineWalkOuptut " << i << " was NaN" << '\n';
    }
    if (std::isnan(walkingEngineStandOutput_->angles[i]))
    {
      std::cout << "WalkingengineStandOuput" << i << " was NaN" << '\n';
    }
    if (fallManagerOutput_->wantToSend && std::isnan(fallManagerOutput_->angles[i]))
    {
      std::cout << "FallManagerOutput_ " << i << " was NaN" << '\n';
    }
    if (std::isnan(poserOutput_->angles[i]))
    {
      std::cout << "PoserOutput " << i << " was NaN" << '\n';
    }
    if (std::isnan(standUpOutput_->angles[i]))
    {
      std::cout << "StandUpOutput " << i << " was NaN" << '\n';
    }

    assert(!std::isnan(angles_[i]));
    assert(!std::isnan(stiffnesses_[i]));
  }
#endif
  robotInterface().setJointAngles(angles_);
  robotInterface().setJointStiffnesses(stiffnesses_);
}
