#pragma once

#include "Data/HeadDamageData.hpp"
#include "Framework/Module.hpp"

class Brain;

class HeadDamageProvider : public Module<HeadDamageProvider, Brain>
{
public:
  ModuleName name__{"HeadDamageProvider"};
  explicit HeadDamageProvider(const ModuleManagerInterface& manager);
  void cycle() override;

private:
  /// Whether the hardware is broken
  /// Speaker
  const Parameter<bool> leftSpeaker_;
  const Parameter<bool> rightSpeaker_;
  /// Microphones
  const Parameter<bool> microFront_;
  const Parameter<bool> microRear_;
  const Parameter<bool> microLeft_;
  const Parameter<bool> microRight_;
  /// Video cameras
  const Parameter<bool> cameraTop_;
  const Parameter<bool> cameraBottom_;
  /// Infra-Red
  const Parameter<bool> infraRedRight_;
  const Parameter<bool> infraRedLeft_;
  /// LEDs
  const Parameter<bool> ledLeftEar_;
  const Parameter<bool> ledRightEar_;
  const Parameter<bool> ledLeftEye_;
  const Parameter<bool> ledRightEye_;
  const Parameter<bool> ledSkull_;
  /// Contact and tactile sensors - Head
  const Parameter<bool> headTouchFront_;
  const Parameter<bool> headTouchMiddle_;
  const Parameter<bool> headTouchRear_;
  /// Used to savely update output
  bool damageStateChanged_;
  Production<HeadDamageData> headDamageData_;
  /// Updates the output for all joints
  void updateState();
};
