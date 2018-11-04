#pragma once

#include "Data/JointCalibrationData.hpp"
#include "Framework/Module.hpp"

class Motion;

class JointCalibrationProvider : public Module<JointCalibrationProvider, Motion>
{
public:
  ModuleName name = "JointCalibrationProvider";
  JointCalibrationProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  // the offsets of all joints
  Parameter<float> headYaw_;
  Parameter<float> headPitch_;
  Parameter<float> lShoulderPitch_;
  Parameter<float> lShoulderRoll_;
  Parameter<float> lElbowYaw_;
  Parameter<float> lElbowRoll_;
  Parameter<float> lWristYaw_;
  Parameter<float> lHand_;
  Parameter<float> lHipYawPitch_;
  Parameter<float> lHipRoll_;
  Parameter<float> lHipPitch_;
  Parameter<float> lKneePitch_;
  Parameter<float> lAnklePitch_;
  Parameter<float> lAnkleRoll_;
  Parameter<float> rHipYawPitch_;
  Parameter<float> rHipRoll_;
  Parameter<float> rHipPitch_;
  Parameter<float> rKneePitch_;
  Parameter<float> rAnklePitch_;
  Parameter<float> rAnkleRoll_;
  Parameter<float> rShoulderPitch_;
  Parameter<float> rShoulderRoll_;
  Parameter<float> rElbowYaw_;
  Parameter<float> rElbowRoll_;
  Parameter<float> rWristYaw_;
  Parameter<float> rHand_;
  /// used to savely update output
  bool configChanged_;
  /// the output of this module, containing the offset of all joints in rad
  Production<JointCalibrationData> jointCalibrationData_;
  /// updates the output for all joints
  void updateOutput();
};
