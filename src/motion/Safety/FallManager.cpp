#include "print.hpp"

#include "Modules/Poses.h"

#include "FallManager.hpp"


FallManager::FallManager(const ModuleManagerInterface& manager)
  : Module(manager)
  , kneeDownMotionFile_(*this, "kneeDownMotionFile")
  , enabled_(*this, "enabled")
  , rapidReachStiffness_(*this, "rapidReachStiffness")
  , motionActivation_(*this)
  , motionRequest_(*this)
  , bodyPose_(*this)
  , cycleInfo_(*this)
  , jointSensorData_(*this)
  , fallManagerOutput_(*this)
  , hot_(false)
  , catchFrontDuration_(*this, "catchFrontDuration", [] {})
  , catchFrontHipPitch_(*this, "catchFrontHipPitch", [this] { catchFrontHipPitch_() *= TO_RAD; })
  , headYawStiffnessThresh_(*this, "headYawStiffnessThresh",
                            [this] { headYawStiffnessThresh_() *= TO_RAD; })
  , headPitchStiffnessThresh_(*this, "headPitchStiffnessThresh",
                              [this] { headPitchStiffnessThresh_() *= TO_RAD; })
  , kneeDown_(*cycleInfo_, *jointSensorData_)
  , timeCatchFrontLastTriggered_(0)
{
  catchFrontHipPitch_() *= TO_RAD;
  headYawStiffnessThresh_() *= TO_RAD;
  headPitchStiffnessThresh_() *= TO_RAD;

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
    fallManagerOutput_->stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 0.7f);
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
  stiffnessController();
}

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
    timeCatchFrontLastTriggered_ = TimePoint::getCurrentTime();
    print("Catch Front!", LogLevel::DEBUG);
  }
  else
  {
    print("Catch Back!", LogLevel::DEBUG);
    timerClock_ = kneeDown_.play();
  }
}

void FallManager::stiffnessController()
{
  // if fall manager is not issuing commands or safe to exit, do nothing
  if (fallManagerOutput_->safeExit || !fallManagerOutput_->wantToSend)
  {
    return;
  }
  // update head joint destinations
  const float headYawDest = fallManagerOutput_->angles[JOINTS::HEAD_YAW];
  const float headPitchDest = fallManagerOutput_->angles[JOINTS::HEAD_PITCH];
  // "control"
  if (std::abs(headYawDest - jointSensorData_->angles[JOINTS::HEAD_YAW]) >
      headYawStiffnessThresh_())
  {
    print("Head Yaw stiffness modified!", LogLevel::DEBUG);
    fallManagerOutput_->stiffnesses[JOINTS::HEAD_YAW] = rapidReachStiffness_();
  }
  if (std::abs(headPitchDest - jointSensorData_->angles[JOINTS::HEAD_PITCH]) >
      headPitchStiffnessThresh_())
  {
    print("Head Pitch stiffness modified!", LogLevel::DEBUG);
    fallManagerOutput_->stiffnesses[JOINTS::HEAD_PITCH] = rapidReachStiffness_();
  }
}
