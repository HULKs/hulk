#include "Motion/Jump/Jump.hpp"

Jump::Jump(const ModuleManagerInterface& manager)
  : Module{manager}
  , actionCommand_{*this}
  , cycleInfo_{*this}
  , jointSensorData_{*this}
  , motionActivation_{*this}
  , poses_{*this}
  , jumpOutput_{*this}
  , squatCatchFront_{*cycleInfo_, *jointSensorData_}
  , stationaryCatchLeft_{*cycleInfo_, *jointSensorData_}
  , stationaryCatchRight_{*cycleInfo_, *jointSensorData_}
  , jumpingCatchLeft_{*cycleInfo_, *jointSensorData_}
  , jumpingCatchRight_{*cycleInfo_, *jointSensorData_}
  , standUpFromGenuflect_{*cycleInfo_, *jointSensorData_}
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
  using BodyMotion = ActionCommand::Body::MotionType;
  using JumpType = JumpOutput::Type;
  const bool incomingJumpRequest = actionCommand_->body().type == BodyMotion::JUMP &&
                                   motionActivation_->activations[BodyMotion::JUMP] == 1.f;
  if (incomingJumpRequest && !isActive_)
  {
    switch (actionCommand_->body().jumpType)
    {
      case JumpType::SQUAT:
        squatCatchFront_.play();
        break;
      case JumpType::TAKE_LEFT:
        stationaryCatchLeft_.play();
        break;
      case JumpType::TAKE_RIGHT:
        stationaryCatchRight_.play();
        break;
      case JumpType::JUMP_LEFT:
        jumpingCatchLeft_.play();
        break;
      case JumpType::JUMP_RIGHT:
        jumpingCatchRight_.play();
        break;
      case JumpType::NONE:
        break;
      default:
        Log<M_MOTION>(LogLevel::ERROR) << "Encountered unhandled JumpType";
        assert(false);
        break;
    }
    previousMotion_ = actionCommand_->body().jumpType;
    isActive_ = true;
  }

  MotionFilePlayer::JointValues values;
  bool wantToSend = false;
  // check if a jump motion file is playing or if the previous angles should be kept
  if (squatCatchFront_.isPlaying())
  {
    values = squatCatchFront_.cycle();
    wantToSend = true;
  }
  else if (stationaryCatchLeft_.isPlaying())
  {
    values = stationaryCatchLeft_.cycle();
    wantToSend = true;
  }
  else if (stationaryCatchRight_.isPlaying())
  {
    values = stationaryCatchRight_.cycle();
    wantToSend = true;
  }
  else if (jumpingCatchLeft_.isPlaying())
  {
    values = jumpingCatchLeft_.cycle();
    wantToSend = true;
  }
  else if (jumpingCatchRight_.isPlaying())
  {
    values = jumpingCatchRight_.cycle();
    wantToSend = true;
  }
  else if (previousMotion_ == actionCommand_->body().jumpType &&
           actionCommand_->body().type == BodyMotion::JUMP)
  {
    // hold previous angles
    values = previousValues_;
    wantToSend = true;
  }
  else if (previousMotion_ == JumpType::SQUAT)
  {
    // initialize stand up after squat
    standUpFromGenuflect_.play();
    previousMotion_ = JumpType::NONE;
  }
  else
  {
    wantToSend = false;
  }

  // check if a stand up motion file is playing
  if (standUpFromGenuflect_.isPlaying())
  {
    values = standUpFromGenuflect_.cycle();
    wantToSend = true;
  }

  // send the appropriate output
  if (wantToSend)
  {
    jumpOutput_->angles = values.angles;
    jumpOutput_->stiffnesses = values.stiffnesses;
    previousValues_ = values;
  }
  else
  {
    jumpOutput_->angles = poses_->angles[Poses::Type::READY];
    jumpOutput_->stiffnesses.fill(0.7f);
    jumpOutput_->safeExit = true;
    isActive_ = false;
  }
}
