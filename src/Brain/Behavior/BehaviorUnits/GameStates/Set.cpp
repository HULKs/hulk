#include "Brain/Behavior/Units.hpp"

ActionCommand set(const DataSet& d)
{
  if (!d.robotPosition.valid)
  {
    Log<M_BRAIN>(LogLevel::WARNING) << "Invalid robot position!";
  }
  return ActionCommand::stand().combineHead(activeVision(d, VisionMode::BALL_TRACKER));
}
