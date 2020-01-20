#include <Modules/NaoProvider.h>
#include <Modules/Poses.h>

#include "SitUp.hpp"
#include "print.hpp"

SitUp::SitUp(const ModuleManagerInterface& manager)
  : Module(manager)
  , sitUpMotionFile_(*this, "sitUpMotionFile")
  , motionRequest_(*this)
  , motionActivation_(*this)
  , cycleInfo_(*this)
  , jointSensorData_(*this)
  , sitUpOutput_(*this)
  , status_(Status::IDLE)
  , sitUpMotion_(*cycleInfo_, *jointSensorData_)
{
  print("sitUp: Initializing module...", LogLevel::INFO);
  sitUpMotion_.loadFromFile(robotInterface().getFileRoot() + "motions/" + sitUpMotionFile_());
}

void SitUp::cycle()
{
  if ((status_ == Status::DONE) || (status_ == Status::IDLE))
  {
    sitUpOutput_->safeExit = true;
  }

  if (motionActivation_->activeMotion == MotionRequest::BodyMotion::SIT_UP &&
      motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::SIT_UP)] == 1.0)
  {
    if (status_ == Status::IDLE)
    {
      status_ = Status::SITTING_UP;
      // initiate movement
      sitUpMotion_.play();
      sitUpOutput_->safeExit = false;
      print("sitUp: Motion starting...", LogLevel::INFO);
    }
  }
  else if (motionActivation_->activeMotion == MotionRequest::BodyMotion::SIT_DOWN)
  {
    status_ = Status::IDLE;
  }
  else
  {
    sitUpOutput_->angles = Poses::getPose(Poses::READY);
    sitUpOutput_->stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 0.7f);
  }

  if (status_ == Status::SITTING_UP)
  {
    MotionFilePlayer::JointValues values;
    bool send = false;

    if (sitUpMotion_.isPlaying())
    {
      values = sitUpMotion_.cycle();
      send = true;
    }
    else
    {
      print("sitUp: Motion done", LogLevel::INFO);
      status_ = Status::DONE;
      sitUpOutput_->safeExit = true;
    }

    if (send)
    {
      sitUpOutput_->angles = values.angles;
      sitUpOutput_->stiffnesses = values.stiffnesses;
    }
  }
}
