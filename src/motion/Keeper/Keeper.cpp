#include "Modules/Poses.h"

#include "Keeper.hpp"


Keeper::Keeper(const ModuleManagerInterface& manager) :
  Module(manager, "Keeper"),
  motionActivation_(*this),
  motionRequest_(*this),
  cycleInfo_(*this),
  jointSensorData_(*this),
  keeperOutput_(*this),
  genuflectCatchFront_(*cycleInfo_, *jointSensorData_),
  stationaryCatchLeft_(*cycleInfo_, *jointSensorData_),
  stationaryCatchRight_(*cycleInfo_, *jointSensorData_),
  jumpingCatchLeft_(*cycleInfo_, *jointSensorData_),
  jumpingCatchRight_(*cycleInfo_, *jointSensorData_),
  standUpFromGenuflect_(*cycleInfo_, *jointSensorData_),
  wasActive_(false),
  previousMotion_(MK_NONE)
{
  std::string motionFileRoot = robotInterface().getFileRoot() + "motions/";

  genuflectCatchFront_.loadFromFile(motionFileRoot + "genuflectCatchFront.motion2");
  stationaryCatchLeft_.loadFromFile(motionFileRoot + "stationaryCatchLeft.motion2");
  stationaryCatchRight_.loadFromFile(motionFileRoot + "stationaryCatchRight.motion2");
  jumpingCatchLeft_.loadFromFile(motionFileRoot + "jumpingCatchLeft.motion2");
  jumpingCatchRight_.loadFromFile(motionFileRoot + "jumpingCatchRight.motion2");
  standUpFromGenuflect_.loadFromFile(motionFileRoot + "standUpFromGenuflect.motion2");
}

void Keeper::cycle()
{

  const bool incomingKeeperRequest = motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::KEEPER)] == 1 &&
    motionRequest_->bodyMotion == MotionRequest::BodyMotion::KEEPER;
  if (incomingKeeperRequest && !wasActive_)
  {
    switch (motionRequest_->keeperData.keep) {
      case MK_TAKE_FRONT:
        genuflectCatchFront_.play();
        break;
      case MK_TAKE_LEFT:
        stationaryCatchLeft_.play();
        break;
      case MK_TAKE_RIGHT:
        stationaryCatchRight_.play();
        break;
      case MK_JUMP_LEFT:
        jumpingCatchLeft_.play();
        break;
      case MK_JUMP_RIGHT:
        jumpingCatchRight_.play();
        break;
      case MK_NONE:
        break;
    }
    previousMotion_ = motionRequest_->keeperData.keep;
    wasActive_ = true;

  }

  MotionFilePlayer::JointValues values;
  bool send = false;
  /// check if a keeper motion file is playing or if the previous angles should be kept
  if (genuflectCatchFront_.isPlaying()) {
    values = genuflectCatchFront_.cycle();
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
  } else if (previousMotion_ == motionRequest_->keeperData.keep && motionRequest_->bodyMotion == MotionRequest::BodyMotion::KEEPER) {
    /// hold previous angles
    values = previousValues_;
    send = true;
  } else if (previousMotion_ == MK_TAKE_FRONT) {
    /// initialize stand up after genuflect
    standUpFromGenuflect_.play();
    previousMotion_ = MK_NONE;
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
    keeperOutput_->angles = values.angles;
    keeperOutput_->stiffnesses = values.stiffnesses;
    previousValues_ = values;
  } else {
    // TODO: test this
    keeperOutput_->angles = Poses::getPose(Poses::READY);
    keeperOutput_->stiffnesses = std::vector<float>(keeperOutput_->angles.size(), 0.7f);
    keeperOutput_->safeExit = true;
    wasActive_ = false;
  }
}
