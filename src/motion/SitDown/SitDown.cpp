#include <Modules/NaoProvider.h>
#include <Modules/Poses.h>

#include "SitDown.hpp"
#include "print.hpp"

SitDown::SitDown(const ModuleManagerInterface& manager)
  : Module(manager)
  , sitDownMotionFile_(*this, "sitDownMotionFile")
  , motionRequest_(*this)
  , motionActivation_(*this)
  , cycleInfo_(*this)
  , jointSensorData_(*this)
  , sitDownOutput_(*this)
  , status_(Status::IDLE)
  , sitDownMotion_(*cycleInfo_, *jointSensorData_)
{
  print("sitDown: Initializing module...", LogLevel::INFO);
  sitDownMotion_.loadFromFile(robotInterface().getFileRoot() + "motions/" + sitDownMotionFile_());
}

void SitDown::cycle()
{
  sitDownOutput_->isSitting = (status_ == Status::DONE);

  if ((status_ == Status::DONE) || (status_ == Status::IDLE))
  {
    sitDownOutput_->safeExit = true;
  }

  if (motionActivation_->activeMotion == MotionRequest::BodyMotion::SIT_DOWN &&
      motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::SIT_DOWN)] == 1.0)
  {
    if (status_ == Status::IDLE)
    {
      status_ = Status::SITTING_DOWN;
      // initiate movement
      sitDownMotion_.play();
      sitDownOutput_->safeExit = false;
      print("sitDown: Motion starting...", LogLevel::INFO);
    }
  }
  else if (motionActivation_->activeMotion == MotionRequest::BodyMotion::SIT_UP)
  {
    status_ = Status::IDLE;
  }
  else
  {
    sitDownOutput_->angles = Poses::getPose(Poses::READY);
    sitDownOutput_->stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 0.7f);
  }

  if (status_ == Status::SITTING_DOWN)
  {
    MotionFilePlayer::JointValues values;
    bool send = false;

    if (sitDownMotion_.isPlaying())
    {
      values = sitDownMotion_.cycle();
      send = true;
    }
    else
    {
      print("sitDown: Motion done", LogLevel::INFO);
      status_ = Status::DONE;
      sitDownOutput_->safeExit = true;
    }

    if (send)
    {
      sitDownOutput_->angles = values.angles;
      sitDownOutput_->stiffnesses = values.stiffnesses;
    }
    else
    {
      sitDownOutput_->stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 0.1f);
    }
  }
}
