#include "Brain/Behavior/Units.hpp"

ActionCommand standUp(const DataSet& d)
{
  if (d.motionState.bodyMotion == ActionCommand::Body::MotionType::STAND_UP && !d.bodyPose.fallen)
  {
    return ActionCommand::stand();
  }
  return ActionCommand::standUp();
}
