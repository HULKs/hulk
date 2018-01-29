#include "print.hpp"

#include "FallManager.hpp"


FallManager::FallManager(const ModuleManagerInterface& manager) :
  Module(manager, "FallManager"),
  catchFrontMotionFile_(*this, "catchFrontMotionFile"),
  kneeDownMotionFile_(*this, "kneeDownMotionFile"),
  enabled_(*this, "enabled"),
  motionActivation_(*this),
  bodyPose_(*this),
  cycleInfo_(*this),
  jointSensorData_(*this),
  fallManagerOutput_(*this),
  hot_(false),
  catchFront_(*cycleInfo_, *jointSensorData_),
  kneeDown_(*cycleInfo_, *jointSensorData_)
{
  std::string motionFileRoot = robotInterface().getFileRoot() + "motions/"; // motionFile path

  // Loading MotionFiles
  catchFront_.loadFromFile(motionFileRoot + catchFrontMotionFile_());
  kneeDown_.loadFromFile(motionFileRoot + kneeDownMotionFile_());
}

void FallManager::cycle()
{
  if (enabled_() && (motionActivation_->activeMotion == MotionRequest::BodyMotion::WALK || motionActivation_->activeMotion == MotionRequest::BodyMotion::STAND)) {
    enableProtection();
  } else {
    disableProtection();
  }

  if (bodyPose_->fallDirection != FallDirection::NOT_FALLING) {
    prepareFalling(bodyPose_->fallDirection);
  }
  if (catchFront_.isPlaying()) {
    MotionFilePlayer::JointValues values = catchFront_.cycle();
    fallManagerOutput_->wantToSend = true;
    fallManagerOutput_->safeExit = false;
    fallManagerOutput_->angles = values.angles;
    fallManagerOutput_->stiffnesses = values.stiffnesses;
  } else if (kneeDown_.isPlaying()) {
    MotionFilePlayer::JointValues values = kneeDown_.cycle();
    fallManagerOutput_->wantToSend = true;
    fallManagerOutput_->safeExit = false;
    fallManagerOutput_->angles = values.angles;
    fallManagerOutput_->stiffnesses = values.stiffnesses;
  } else {
    fallManagerOutput_->wantToSend = false;
    fallManagerOutput_->safeExit = true;
  }
}

/**
 * @brief call to enable protection
 */
void FallManager::enableProtection(){
  hot_ = true;
  print("FallManager enabled", LogLevel::INFO);
}

/**
 * @brief call to disable protection
 */
void FallManager::disableProtection(){
  hot_ = false;
  print("FallManager disabled", LogLevel::INFO);
}

/**
 * @brief Reacting on the falling detected by OnCycle
 * TODO: individual reaction on falling directions
 */
void FallManager::prepareFalling(const FallDirection fallDirection)
{
  if (!hot_) { /// Only react if hot
    print("Falling - but FallManager disabled", LogLevel::DEBUG);
    return;
  }

  disableProtection();

  if(fallDirection == FallDirection::FRONT) { // accomplish reaction move depenting on tendency of falling
    timerClock_ = catchFront_.play();
    print("Catch Front!", LogLevel::DEBUG);
  } else {
    print("Catch Back!", LogLevel::DEBUG);
    timerClock_ = kneeDown_.play();
  }
}
