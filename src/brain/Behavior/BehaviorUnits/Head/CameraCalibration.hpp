#pragma once
#include "Behavior/Units.hpp"

ActionCommand::Head CameraCalibrationLook(const DataSet& d)
{
  return ActionCommand::Head::angles(d.parameters.calibrationHeadYaw(), d.parameters.calibrationHeadPitch());
}
