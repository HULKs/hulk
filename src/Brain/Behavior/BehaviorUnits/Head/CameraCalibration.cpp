#include "Brain/Behavior/Units.hpp"

ActionCommand::Head cameraCalibrationLook(const DataSet& d)
{
  return ActionCommand::Head::angles(d.parameters.calibrationHeadYaw(),
                                     d.parameters.calibrationHeadPitch());
}
