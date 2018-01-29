#pragma once
#include "Behavior/Units.hpp"

ActionCommand standUp(const DataSet& d)
{
  if (d.motionState.bodyMotion == MotionRequest::BodyMotion::STAND_UP && !d.bodyPose.fallen)
  {
    return ActionCommand::stand();
  }
  else
  {
    return ActionCommand::standUp();
  }
}
