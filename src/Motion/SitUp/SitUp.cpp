#include "Motion/SitUp/SitUp.hpp"
#include "Framework/Log/Log.hpp"

SitUp::SitUp(const ModuleManagerInterface& manager)
  : Module{manager}
  , actionCommand_{*this}
  , cycleInfo_{*this}
  , jointSensorData_{*this}
  , motionActivation_{*this}
  , poses_{*this}
  , sitUpOutput_{*this}
  , sitUpMotionFile_{*this, "sitUpMotionFile"}
  , state_{State::IDLE}
  , sitUpMotion_{*cycleInfo_, *jointSensorData_}
{
  Log<M_MOTION>(LogLevel::INFO) << "SitUp: Initializing module...";
  sitUpMotion_.loadFromFile(robotInterface().getFileRoot() + "motions/" + sitUpMotionFile_());
}

void SitUp::cycle()
{
  using BodyMotion = ActionCommand::Body::MotionType;

  // handle state transitions
  if (state_ == State::IDLE && motionActivation_->activeMotion == BodyMotion::SIT_UP &&
      motionActivation_->activations[BodyMotion::SIT_UP] == 1.f)
  {
    state_ = State::SITTING_UP;
    // initiate movement
    sitUpMotion_.play();
    Log<M_MOTION>(LogLevel::INFO) << "SitUp: Motion starting...";
  }
  else if (state_ == State::SITTING_UP && !sitUpMotion_.isPlaying())
  {
    state_ = State::DONE;
  }
  else if (state_ == State::DONE && motionActivation_->activations[BodyMotion::SIT_UP] == 0.f)
  {
    state_ = State::IDLE;
  }

  // output based on state
  if (state_ == State::IDLE)
  {
    sitUpOutput_->angles = poses_->angles[Poses::Type::SITTING];
    sitUpOutput_->stiffnesses.fill(0.5f);
    sitUpOutput_->safeExit = false;
    sitUpOutput_->valid = true;
  }
  else if (state_ == State::SITTING_UP)
  {
    const MotionFilePlayer::JointValues values = sitUpMotion_.cycle();
    sitUpOutput_->angles = values.angles;
    sitUpOutput_->stiffnesses = values.stiffnesses;
    sitUpOutput_->safeExit = false;
    sitUpOutput_->valid = true;
  }
  else if (state_ == State::DONE)
  {
    sitUpOutput_->angles = sitUpMotion_.cycle().angles;
    sitUpOutput_->stiffnesses.fill(0.7f);
    sitUpOutput_->safeExit = true;
    sitUpOutput_->valid = true;
  }
}
