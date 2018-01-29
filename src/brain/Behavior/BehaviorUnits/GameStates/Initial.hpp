#pragma once
#include "Behavior/Units.hpp"

ActionCommand initial(const DataSet& d)
{
  if (d.gameControllerState.chestButtonWasPressedInInitial)
  {
    if (d.parameters.isCameraCalibration())
    {
      return ActionCommand::stand().combineHead(CameraCalibrationLook(d));
    }
    
    return ActionCommand::penalized();
  }
  else
  {
    return ActionCommand::dead();
  }
}
