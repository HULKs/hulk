#include "print.hpp"

#include "Modules/Poses.h"

#include "FallManager.hpp"


FallManager::FallManager(const ModuleManagerInterface& manager)
  : Module(manager)
  , kneeDownMotionFile_(*this, "kneeDownMotionFile")
  , enabled_(*this, "enabled")
  , motionActivation_(*this)
  , motionRequest_(*this)
  , bodyPose_(*this)
  , cycleInfo_(*this)
  , jointSensorData_(*this)
  , fallManagerOutput_(*this)
  , hot_(false)
  , catchFrontDuration_(*this, "catchFrontDuration", [] {})
  , catchFrontHipPitch_(*this, "catchFrontHipPitch", [this] { catchFrontHipPitch_() *= TO_RAD; })
  , kneeDown_(*cycleInfo_, *jointSensorData_)
{
  catchFrontHipPitch_() *= TO_RAD;

  std::string motionFileRoot = robotInterface().getFileRoot() + "motions/"; // motionFile path

  // Loading MotionFile
  kneeDown_.loadFromFile(motionFileRoot + kneeDownMotionFile_());

  lastAngles_ = Poses::getPose(Poses::READY);
}

void FallManager::cycle()
{
  hot_ = enabled_() && (motionRequest_->bodyMotion == MotionRequest::BodyMotion::WALK ||
      motionRequest_->bodyMotion == MotionRequest::BodyMotion::STAND);

  if (bodyPose_->fallDirection != FallDirection::NOT_FALLING)
  {
    prepareFalling(bodyPose_->fallDirection);
  }
  if (!catchFrontInterpolator_.finished())
  {
    fallManagerOutput_->wantToSend = true;
    fallManagerOutput_->safeExit = false;
    fallManagerOutput_->angles = catchFrontInterpolator_.step(10);
    fallManagerOutput_->stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 0.7);
  }
  else if (kneeDown_.isPlaying())
  {
    MotionFilePlayer::JointValues values = kneeDown_.cycle();
    fallManagerOutput_->wantToSend = true;
    fallManagerOutput_->safeExit = false;
    fallManagerOutput_->angles = values.angles;
    fallManagerOutput_->stiffnesses = values.stiffnesses;
  }
  else
  {
    fallManagerOutput_->angles = lastAngles_;
    fallManagerOutput_->stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 0.7f);
    fallManagerOutput_->wantToSend = false;
    fallManagerOutput_->safeExit = true;
  }
  lastAngles_ = fallManagerOutput_->angles;
}

/**
 * @brief Reacting on the falling detected by OnCycle
 * TODO: individual reaction on falling directions
 */
void FallManager::prepareFalling(const FallDirection fallDirection)
{
  // Only react if hot
  if (!hot_)
  {
    print("Falling - but FallManager disabled", LogLevel::DEBUG);
    return;
  }

  // disable protection
  hot_ = false;

  // accomplish reaction move depenting on tendency of falling
  if (fallDirection == FallDirection::FRONT)
  {
    std::vector<float> catchFrontAngles = Poses::getPose(Poses::READY);
    catchFrontAngles[JOINTS::HEAD_PITCH] = -38.5 * TO_RAD; // set the head pitch to the minimum
    // set hip pitches
    catchFrontAngles[JOINTS::L_HIP_PITCH] = catchFrontHipPitch_();
    catchFrontAngles[JOINTS::R_HIP_PITCH] = catchFrontHipPitch_();
    catchFrontInterpolator_.reset(jointSensorData_->getBodyAngles(), catchFrontAngles,
                                  catchFrontDuration_());
    print("Catch Front!", LogLevel::DEBUG);
  }
  else
  {
    print("Catch Back!", LogLevel::DEBUG);
    timerClock_ = kneeDown_.play();
  }
}
