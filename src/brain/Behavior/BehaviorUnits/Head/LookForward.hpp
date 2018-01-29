#pragma once
#include "Behavior/Units.hpp"

ActionCommand::Head lookForward(const DataSet& d)
{
  return ActionCommand::Head::angles(0, d.parameters.lookAroundOuterPosition()[1]);
}
