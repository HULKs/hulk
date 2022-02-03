#include "Motion/Safety/FallManager.hpp"
#include "Framework/Log/Log.hpp"
#include <type_traits>

FallManager::FallManager(const ModuleManagerInterface& manager)
  : Module{manager}
  , actionCommand_{*this}
  , bodyPose_{*this}
  , cycleInfo_{*this}
  , jointSensorData_{*this}
  , motionActivation_{*this}
  , poses_{*this}
  , fallManagerOutput_{*this}
  , kneeDownMotionFile_{*this, "kneeDownMotionFile"}
  , enabled_{*this, "enabled"}
  , rapidReachStiffness_{*this, "rapidReachStiffness"}
  , catchFrontDuration_{*this, "catchFrontDuration", [] {}}
  , catchFrontHipPitch_{*this, "catchFrontHipPitch", [this] { catchFrontHipPitch_() *= TO_RAD; }}
  , headYawStiffnessThresh_{*this, "headYawStiffnessThresh",
                            [this] { headYawStiffnessThresh_() *= TO_RAD; }}
  , headPitchStiffnessThresh_{*this, "headPitchStiffnessThresh",
                              [this] { headPitchStiffnessThresh_() *= TO_RAD; }}
  , kneeDown_{*cycleInfo_, *jointSensorData_}
{
  catchFrontHipPitch_() *= TO_RAD;
  headYawStiffnessThresh_() *= TO_RAD;
  headPitchStiffnessThresh_() *= TO_RAD;

  std::string motionFileRoot = robotInterface().getFileRoot() + "motions/"; // motionFile path

  // Loading MotionFile
  kneeDown_.loadFromFile(motionFileRoot + kneeDownMotionFile_());
}

void FallManager::cycle()
{
  hot_ = enabled_() && (actionCommand_->body().type == ActionCommand::Body::MotionType::WALK ||
                        actionCommand_->body().type == ActionCommand::Body::MotionType::STAND);

  if (bodyPose_->fallDirection != BodyPose::FallDirection::NOT_FALLING)
  {
    prepareFalling(bodyPose_->fallDirection);
  }
  if (!catchFrontInterpolator_.isFinished())
  {
    fallManagerOutput_->wantToSend = true;
    fallManagerOutput_->safeExit = false;
    fallManagerOutput_->angles = {catchFrontInterpolator_.step(cycleInfo_->cycleTime)};
    fallManagerOutput_->stiffnesses.fill(0.7f);
  }
  else if (kneeDown_.isPlaying())
  {
    fallManagerOutput_->wantToSend = true;
    fallManagerOutput_->safeExit = false;
    MotionFilePlayer::JointValues values = kneeDown_.cycle();
    fallManagerOutput_->angles = values.angles;
    fallManagerOutput_->stiffnesses = values.stiffnesses;
  }
  else
  {
    fallManagerOutput_->wantToSend = false;
    fallManagerOutput_->safeExit = true;
    fallManagerOutput_->angles = lastAngles_;
    fallManagerOutput_->stiffnesses.fill(0.7f);
  }
  lastAngles_ = fallManagerOutput_->angles;
  stiffnessController();
}

void FallManager::prepareFalling(const BodyPose::FallDirection fallDirection)
{
  // Only react if hot
  if (!hot_)
  {
    Log<M_MOTION>(LogLevel::DEBUG) << "Falling - but FallManager disabled";
    return;
  }

  // disable protection
  hot_ = false;

  // accomplish reaction move depenting on tendency of falling
  if (fallDirection == BodyPose::FallDirection::FRONT)
  {
    auto catchFrontAngles = poses_->angles[Poses::Type::READY];
    catchFrontAngles[Joints::HEAD_PITCH] = robotMetrics().minRange(Joints::HEAD_PITCH);
    // set hip pitches
    catchFrontAngles[Joints::L_HIP_PITCH] = catchFrontHipPitch_();
    catchFrontAngles[Joints::R_HIP_PITCH] = catchFrontHipPitch_();
    catchFrontInterpolator_.reset(jointSensorData_->getBodyAngles(), catchFrontAngles,
                                  catchFrontDuration_());
    Log<M_MOTION>(LogLevel::DEBUG) << "Catch Front";
  }
  else
  {
    Log<M_MOTION>(LogLevel::DEBUG) << "Catch Back";
    kneeDown_.play();
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
  const float headYawDest = fallManagerOutput_->angles[Joints::HEAD_YAW];
  const float headPitchDest = fallManagerOutput_->angles[Joints::HEAD_PITCH];
  // "control"
  if (std::abs(headYawDest - jointSensorData_->angles[Joints::HEAD_YAW]) >
      headYawStiffnessThresh_())
  {
    Log<M_MOTION>(LogLevel::DEBUG) << "Head Yaw stiffness modified";
    fallManagerOutput_->stiffnesses[Joints::HEAD_YAW] = rapidReachStiffness_();
  }
  if (std::abs(headPitchDest - jointSensorData_->angles[Joints::HEAD_PITCH]) >
      headPitchStiffnessThresh_())
  {
    Log<M_MOTION>(LogLevel::DEBUG) << "Head Pitch stiffness modified";
    fallManagerOutput_->stiffnesses[Joints::HEAD_PITCH] = rapidReachStiffness_();
  }
}
