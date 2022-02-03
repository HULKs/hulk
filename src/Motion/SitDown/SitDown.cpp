#include "Motion/SitDown/SitDown.hpp"
#include "Framework/Log/Log.hpp"

SitDown::SitDown(const ModuleManagerInterface& manager)
  : Module{manager}
  , actionCommand_{*this}
  , cycleInfo_{*this}
  , jointSensorData_{*this}
  , motionActivation_{*this}
  , poses_{*this}
  , sitDownOutput_{*this}
  , sitDownMotionFile_{*this, "sitDownMotionFile"}
  , status_{Status::IDLE}
  , sitDownMotion_{*cycleInfo_, *jointSensorData_}
{
  Log<M_MOTION>(LogLevel::INFO) << "SitDown: Initializing module...";
  sitDownMotion_.loadFromFile(robotInterface().getFileRoot() + "motions/" + sitDownMotionFile_());
}

void SitDown::cycle()
{
  using BodyMotion = ActionCommand::Body::MotionType;

  // handle state transitions
  if (status_ == Status::IDLE && motionActivation_->activeMotion == BodyMotion::SIT_DOWN &&
      motionActivation_->activations[BodyMotion::SIT_DOWN] == 1.f)
  {
    status_ = Status::SITTING_DOWN;
    // initiate movement
    sitDownMotion_.play();
    Log<M_MOTION>(LogLevel::INFO) << "SitDown: Motion starting...";
  }
  else if (status_ == Status::SITTING_DOWN && !sitDownMotion_.isPlaying())
  {
    Log<M_MOTION>(LogLevel::INFO) << "SitDown: Motion done";
    status_ = Status::DONE;
  }
  else if (status_ == Status::DONE && motionActivation_->activations[BodyMotion::SIT_DOWN] == 0.f)
  {
    status_ = Status::IDLE;
  }

  // output based on state
  if (status_ == Status::IDLE)
  {
    sitDownOutput_->isSitting = false;
    sitDownOutput_->angles = poses_->angles[Poses::Type::READY];
    sitDownOutput_->stiffnesses.fill(0.7f);
    sitDownOutput_->safeExit = false;
    sitDownOutput_->valid = true;
  }
  else if (status_ == Status::SITTING_DOWN)
  {
    sitDownOutput_->isSitting = false;
    MotionFilePlayer::JointValues values{sitDownMotion_.cycle()};
    sitDownOutput_->angles = values.angles;
    sitDownOutput_->stiffnesses = values.stiffnesses;
    sitDownOutput_->safeExit = false;
    sitDownOutput_->valid = true;
  }
  else if (status_ == Status::DONE)
  {
    sitDownOutput_->isSitting = true;
    sitDownOutput_->angles = sitDownMotion_.cycle().angles;
    sitDownOutput_->stiffnesses.fill(0.1f);
    sitDownOutput_->safeExit = true;
    sitDownOutput_->valid = true;
  }
}
