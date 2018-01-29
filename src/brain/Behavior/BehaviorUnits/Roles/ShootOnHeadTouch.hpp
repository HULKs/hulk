#pragma once
#include "Behavior/Units.hpp"

ActionCommand shootOnHeadTouch(const DataSet& d)
{
  const bool frontHeadTouched = d.buttonData.buttons[keys::sensor::switches::SWITCH_HEAD_FRONT] > 0.1;
  const bool rearHeadTouched = d.buttonData.buttons[keys::sensor::switches::SWITCH_HEAD_REAR] > 0.1;
  if (frontHeadTouched)
  {
    return kickLeft(d);
  }
  else if (rearHeadTouched)
  {
    return kickRight(d);
  }
  return ActionCommand::stand();
}
