#pragma once

#include "Data/BodyDamageData.hpp"
#include "Framework/Module.hpp"

class Motion;

class BodyDamageProvider : public Module<BodyDamageProvider, Motion>
{
public:
  ModuleName name = "BodyDamageProvider";
  BodyDamageProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  /// Whether the hardware is broken
  /// Joints
  Parameter<bool> headYaw_;
  Parameter<bool> headPitch_;
  Parameter<bool> lShoulderPitch_;
  Parameter<bool> lShoulderRoll_;
  Parameter<bool> lElbowYaw_;
  Parameter<bool> lElbowRoll_;
  Parameter<bool> lWristYaw_;
  Parameter<bool> lHand_;
  Parameter<bool> lHipYawPitch_;
  Parameter<bool> lHipRoll_;
  Parameter<bool> lHipPitch_;
  Parameter<bool> lKneePitch_;
  Parameter<bool> lAnklePitch_;
  Parameter<bool> lAnkleRoll_;
  Parameter<bool> rHipYawPitch_;
  Parameter<bool> rHipRoll_;
  Parameter<bool> rHipPitch_;
  Parameter<bool> rKneePitch_;
  Parameter<bool> rAnklePitch_;
  Parameter<bool> rAnkleRoll_;
  Parameter<bool> rShoulderPitch_;
  Parameter<bool> rShoulderRoll_;
  Parameter<bool> rElbowYaw_;
  Parameter<bool> rElbowRoll_;
  Parameter<bool> rWristYaw_;
  Parameter<bool> rHand_;
  /// FSRs
  Parameter<bool> lFsrFL_;
  Parameter<bool> lFsrFR_;
  Parameter<bool> lFsrRL_;
  Parameter<bool> lFsrRR_;
  Parameter<bool> rFsrFL_;
  Parameter<bool> rFsrFR_;
  Parameter<bool> rFsrRL_;
  Parameter<bool> rFsrRR_;
  /// Inertial unit
  Parameter<bool> accelerometer_;
  Parameter<bool> gyrometer_;
  /// Sonars
  Parameter<bool> uSLeft_;
  Parameter<bool> uSRight_;
  /// Contact and tactile sensors - Chest button
  Parameter<bool> chestButton_;
  /// Contact and tactile sensors - Hand
  Parameter<bool> lHandTouchLeft_;
  Parameter<bool> lHandTouchBack_;
  Parameter<bool> lHandTouchRight_;
  Parameter<bool> rHandTouchLeft;
  Parameter<bool> rHandTouchBack;
  Parameter<bool> rHandTouchRight;
  /// Contact and tactile sensors - Foot
  Parameter<bool> bumperLFootLeft_;
  Parameter<bool> bumperLFootRight_;
  Parameter<bool> bumperRFootLeft_;
  Parameter<bool> bumperRFootRight_;
  /// Used to savely update output
  bool damageStateChanged_;
  Production<BodyDamageData> bodyDamageData_;
  /// Updates the output for all joints
  void updateState();
};
