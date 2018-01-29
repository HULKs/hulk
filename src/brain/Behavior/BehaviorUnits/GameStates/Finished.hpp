#pragma once
#include "Behavior/Units.hpp"

ActionCommand finished(const DataSet& d)
{
  return ActionCommand::penalized();
}
