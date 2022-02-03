#include "Brain/Behavior/Units.hpp"

ActionCommand initial(const DataSet& d)
{
  if (d.gameControllerState.chestButtonWasPressedInInitial)
  {
    if (d.parameters.isCameraCalibration())
    {
      return ActionCommand::stand().combineHead(cameraCalibrationLook(d));
    }
    return ActionCommand::penalized();
  }
  return ActionCommand::dead();
}
