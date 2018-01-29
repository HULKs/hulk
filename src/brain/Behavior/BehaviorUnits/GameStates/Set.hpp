#pragma once
#include "Behavior/Units.hpp"

ActionCommand set(const DataSet& d)
{
  return ActionCommand::stand().combineHead(trackBall(d));
}
