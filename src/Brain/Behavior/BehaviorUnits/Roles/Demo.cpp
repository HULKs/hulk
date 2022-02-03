#include "Brain/Behavior/Units.hpp"

ActionCommand demo(const DataSet& d)
{
  return ActionCommand::stand().combineHead(activeVision(d, VisionMode::BALL_TRACKER));
}
