#pragma once

#include "Behavior/Units.hpp"
#include "Tools/Math/Angle.hpp"


ActionCommand set(const DataSet& d)
{
  if (!d.robotPosition.valid)
  {
    Log(LogLevel::WARNING) << "Invalid robot position!";
  }
  return ActionCommand::stand().combineHead(activeVision(d, VisionMode::BALL_TRACKER));
}
