#pragma once

#include "Data/HeadDamageData.hpp"
#include "Framework/Module.hpp"

class Brain;

class HeadDamageProvider : public Module<HeadDamageProvider, Brain>
{
public:
  ModuleName name = "HeadDamageProvider";
  HeadDamageProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  /// Whether the hardware is broken
  /// Speaker
  Parameter<bool> leftSpeaker_;
  Parameter<bool> rightSpeaker_;
  /// Microphones
  Parameter<bool> microFront_;
  Parameter<bool> microRear_;
  Parameter<bool> microLeft_;
  Parameter<bool> microRight_;
  /// Video cameras
  Parameter<bool> cameraTop_;
  Parameter<bool> cameraBottom_;
  /// Infra-Red
  Parameter<bool> infraRedRight_;
  Parameter<bool> infraRedLeft_;
  /// LEDs
  Parameter<bool> rLED0_;
  Parameter<bool> rLED1_;
  Parameter<bool> rLED2_;
  Parameter<bool> rLED3_;
  Parameter<bool> rLED4_;
  Parameter<bool> rLED5_;
  Parameter<bool> rLED6_;
  Parameter<bool> rLED7_;
  Parameter<bool> lLED0_;
  Parameter<bool> lLED1_;
  Parameter<bool> lLED2_;
  Parameter<bool> lLED3_;
  Parameter<bool> lLED4_;
  Parameter<bool> lLED5_;
  Parameter<bool> lLED6_;
  Parameter<bool> lLED7_;
  /// Contact and tactile sensors - Head
  Parameter<bool> headTouchFront_;
  Parameter<bool> headTouchMiddle_;
  Parameter<bool> headTouchRear_;
  /// Used to savely update output
  bool damageStateChanged_;
  Production<HeadDamageData> headDamageData_;
  /// Updates the output for all joints
  void updateState();
};
