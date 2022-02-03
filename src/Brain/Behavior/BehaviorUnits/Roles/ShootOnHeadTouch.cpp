#include "Brain/Behavior/Units.hpp"

ActionCommand shootOnHeadTouch(const DataSet& d)
{
  if (d.buttonData.switches.isHeadFrontPressed) // front head touched
  {
    return kickLeft(d);
  }
  if (d.buttonData.switches.isHeadRearPressed) // rear head touched
  {
    return kickRight(d);
  }
  return ActionCommand::stand();
}
