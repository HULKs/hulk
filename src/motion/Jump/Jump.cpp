#include "Modules/Poses.h"

#include "Jump.hpp"


Jump::Jump(const ModuleManagerInterface& manager) :
  Module(manager),
  motionActivation_(*this),
  motionRequest_(*this),
  cycleInfo_(*this),
  jointSensorData_(*this),
  jumpOutput_(*this),
  squatCatchFront_(*cycleInfo_, *jointSensorData_),
  stationaryCatchLeft_(*cycleInfo_, *jointSensorData_),
  stationaryCatchRight_(*cycleInfo_, *jointSensorData_),
  jumpingCatchLeft_(*cycleInfo_, *jointSensorData_),
  jumpingCatchRight_(*cycleInfo_, *jointSensorData_),
  standUpFromGenuflect_(*cycleInfo_, *jointSensorData_),
  wasActive_(false),
  previousMotion_(NONE)
{
  std::string motionFileRoot = robotInterface().getFileRoot() + "motions/";

  squatCatchFront_.loadFromFile(motionFileRoot + "squatCatchFront.motion2");
  stationaryCatchLeft_.loadFromFile(motionFileRoot + "stationaryCatchLeft.motion2");
  stationaryCatchRight_.loadFromFile(motionFileRoot + "stationaryCatchRight.motion2");
  jumpingCatchLeft_.loadFromFile(motionFileRoot + "jumpingCatchLeft.motion2");
  jumpingCatchRight_.loadFromFile(motionFileRoot + "jumpingCatchRight.motion2");
  standUpFromGenuflect_.loadFromFile(motionFileRoot + "standUpFromSquat.motion2");
}

void Jump::cycle()
{

  const bool incomingJumpRequest = motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::JUMP)] == 1 &&
    motionRequest_->bodyMotion == MotionRequest::BodyMotion::JUMP;
  if (incomingJumpRequest && !wasActive_)
  {
    switch (motionRequest_->jumpData.keep) {
      case SQUAT:
        squatCatchFront_.play();
        break;
      case TAKE_LEFT:
        stationaryCatchLeft_.play();
        break;
      case TAKE_RIGHT:
        stationaryCatchRight_.play();
        break;
      case JUMP_LEFT:
        jumpingCatchLeft_.play();
        break;
      case JUMP_RIGHT:
        jumpingCatchRight_.play();
        break;
      case NONE:
        break;
    }
    previousMotion_ = motionRequest_->jumpData.keep;
    wasActive_ = true;

  }

  MotionFilePlayer::JointValues values;
  bool send = false;
  /// check if a jump motion file is playing or if the previous angles should be kept
  if (squatCatchFront_.isPlaying()) {
    values = squatCatchFront_.cycle();
    send = true;
  } else if (stationaryCatchLeft_.isPlaying()) {
    values = stationaryCatchLeft_.cycle();
    send = true;
  } else if (stationaryCatchRight_.isPlaying()) {
    values = stationaryCatchRight_.cycle();
    send = true;
  } else if (jumpingCatchLeft_.isPlaying()) {
    values = jumpingCatchLeft_.cycle();
    send = true;
  } else if (jumpingCatchRight_.isPlaying()) {
    values = jumpingCatchRight_.cycle();
    send = true;
  } else if (previousMotion_ == motionRequest_->jumpData.keep && motionRequest_->bodyMotion == MotionRequest::BodyMotion::JUMP) {
    /// hold previous angles
    values = previousValues_;
    send = true;
  } else if (previousMotion_ == SQUAT) {
    /// initialize stand up after squat
    standUpFromGenuflect_.play();
    previousMotion_ = NONE;
  } else {
    send = false;
  }

  /// check if a stand up motion file is playing
  if (standUpFromGenuflect_.isPlaying())
  {
    values = standUpFromGenuflect_.cycle();
    send = true;
  }

  /// send the appropriate output
  if (send) {
    jumpOutput_->angles = values.angles;
    jumpOutput_->stiffnesses = values.stiffnesses;
    previousValues_ = values;
  } else {
    // TODO: test this
    jumpOutput_->angles = Poses::getPose(Poses::READY);
    jumpOutput_->stiffnesses = std::vector<float>(jumpOutput_->angles.size(), 0.7f);
    jumpOutput_->safeExit = true;
    wasActive_ = false;
  }
}
