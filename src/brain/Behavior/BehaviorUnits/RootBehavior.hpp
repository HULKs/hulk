#pragma once
#include "Behavior/Units.hpp"

ActionCommand rootBehavior(const DataSet& d)
{
  // If the NAO does not have foot contact it overrides the left LED with pink.
  const bool high = !d.bodyPose.footContact;
  if (d.gameControllerState.penalty == Penalty::NONE)
  {
    return high ? notPenalized(d)
                      .combineHead(activeVision(d, VisionMode::LOOK_FORWARD))
                      .combineLeftLED(ActionCommand::LED::pink())
                : notPenalized(d);
  }
  else
  {
    return high ? ActionCommand::penalized().combineLeftLED(ActionCommand::LED::pink())
                : ActionCommand::penalized();
  }
}
