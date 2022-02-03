#pragma once

#include "Data/BodyDamageData.hpp"
#include "Framework/Module.hpp"

class Motion;

class BodyDamageProvider : public Module<BodyDamageProvider, Motion>
{
public:
  ModuleName name__{"BodyDamageProvider"};
  explicit BodyDamageProvider(const ModuleManagerInterface& manager);
  void cycle() override;

private:
  /// Whether the hardware is broken
  /// Joints
  const Parameter<bool> headYaw_;
  const Parameter<bool> headPitch_;
  const Parameter<bool> lShoulderPitch_;
  const Parameter<bool> lShoulderRoll_;
  const Parameter<bool> lElbowYaw_;
  const Parameter<bool> lElbowRoll_;
  const Parameter<bool> lWristYaw_;
  const Parameter<bool> lHand_;
  const Parameter<bool> lHipYawPitch_;
  const Parameter<bool> lHipRoll_;
  const Parameter<bool> lHipPitch_;
  const Parameter<bool> lKneePitch_;
  const Parameter<bool> lAnklePitch_;
  const Parameter<bool> lAnkleRoll_;
  const Parameter<bool> rHipYawPitch_;
  const Parameter<bool> rHipRoll_;
  const Parameter<bool> rHipPitch_;
  const Parameter<bool> rKneePitch_;
  const Parameter<bool> rAnklePitch_;
  const Parameter<bool> rAnkleRoll_;
  const Parameter<bool> rShoulderPitch_;
  const Parameter<bool> rShoulderRoll_;
  const Parameter<bool> rElbowYaw_;
  const Parameter<bool> rElbowRoll_;
  const Parameter<bool> rWristYaw_;
  const Parameter<bool> rHand_;
  /// FSRs
  const Parameter<bool> lFsrFL_;
  const Parameter<bool> lFsrFR_;
  const Parameter<bool> lFsrRL_;
  const Parameter<bool> lFsrRR_;
  const Parameter<bool> rFsrFL_;
  const Parameter<bool> rFsrFR_;
  const Parameter<bool> rFsrRL_;
  const Parameter<bool> rFsrRR_;
  /// Inertial unit
  const Parameter<bool> accelerometer_;
  const Parameter<bool> gyrometer_;
  /// Sonars
  const Parameter<bool> uSLeft_;
  const Parameter<bool> uSRight_;
  /// Contact and tactile sensors - Chest button
  const Parameter<bool> chestButton_;
  /// Contact and tactile sensors - Hand
  const Parameter<bool> lHandTouchLeft_;
  const Parameter<bool> lHandTouchBack_;
  const Parameter<bool> lHandTouchRight_;
  const Parameter<bool> rHandTouchLeft_;
  const Parameter<bool> rHandTouchBack_;
  const Parameter<bool> rHandTouchRight_;
  /// Contact and tactile sensors - Foot
  const Parameter<bool> bumperLFootLeft_;
  const Parameter<bool> bumperLFootRight_;
  const Parameter<bool> bumperRFootLeft_;
  const Parameter<bool> bumperRFootRight_;

  const Parameter<bool> ledChest_;
  const Parameter<bool> ledLeftFoot_;
  const Parameter<bool> ledRightFoot_;
  /// Used to savely update output
  bool damageStateChanged_;
  Production<BodyDamageData> bodyDamageData_;
  /// Updates the output for all joints
  void updateState();
};
