#include "HeadDamageProvider.hpp"

HeadDamageProvider::HeadDamageProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , leftSpeaker_(*this, "leftSpeaker", [this]() { damageStateChanged_ = true; })
  , rightSpeaker_(*this, "rightSpeaker", [this]() { damageStateChanged_ = true; })
  , microFront_(*this, "microFront", [this]() { damageStateChanged_ = true; })
  , microRear_(*this, "microRear", [this]() { damageStateChanged_ = true; })
  , microLeft_(*this, "microLeft", [this]() { damageStateChanged_ = true; })
  , microRight_(*this, "microRight", [this]() { damageStateChanged_ = true; })
  , cameraTop_(*this, "cameraTop", [this]() { damageStateChanged_ = true; })
  , cameraBottom_(*this, "cameraBottom", [this]() { damageStateChanged_ = true; })
  , infraRedRight_(*this, "infraRedRight", [this]() { damageStateChanged_ = true; })
  , infraRedLeft_(*this, "infraRedLeft", [this]() { damageStateChanged_ = true; })
  , rLED0_(*this, "rLED0", [this]() { damageStateChanged_ = true; })
  , rLED1_(*this, "rLED1", [this]() { damageStateChanged_ = true; })
  , rLED2_(*this, "rLED2", [this]() { damageStateChanged_ = true; })
  , rLED3_(*this, "rLED3", [this]() { damageStateChanged_ = true; })
  , rLED4_(*this, "rLED4", [this]() { damageStateChanged_ = true; })
  , rLED5_(*this, "rLED5", [this]() { damageStateChanged_ = true; })
  , rLED6_(*this, "rLED6", [this]() { damageStateChanged_ = true; })
  , rLED7_(*this, "rLED7", [this]() { damageStateChanged_ = true; })
  , lLED0_(*this, "lLED0", [this]() { damageStateChanged_ = true; })
  , lLED1_(*this, "lLED1", [this]() { damageStateChanged_ = true; })
  , lLED2_(*this, "lLED2", [this]() { damageStateChanged_ = true; })
  , lLED3_(*this, "lLED3", [this]() { damageStateChanged_ = true; })
  , lLED4_(*this, "lLED4", [this]() { damageStateChanged_ = true; })
  , lLED5_(*this, "lLED5", [this]() { damageStateChanged_ = true; })
  , lLED6_(*this, "lLED6", [this]() { damageStateChanged_ = true; })
  , lLED7_(*this, "lLED7", [this]() { damageStateChanged_ = true; })
  , headTouchFront_(*this, "headTouchFront", [this]() { damageStateChanged_ = true; })
  , headTouchMiddle_(*this, "headTouchMiddle", [this]() { damageStateChanged_ = true; })
  , headTouchRear_(*this, "headTouchRear", [this]() { damageStateChanged_ = true; })
  , damageStateChanged_(true)
  , headDamageData_(*this)
{
}

void HeadDamageProvider::cycle()
{
  if (damageStateChanged_)
  {
    updateState();
    damageStateChanged_ = false;
  }
}

void HeadDamageProvider::updateState()
{
  /// Speaker
  headDamageData_->damagedSpeakers[SPEAKERS::LEFT] = leftSpeaker_();
  headDamageData_->damagedSpeakers[SPEAKERS::RIGHT] = rightSpeaker_();
  /// Microphones
  headDamageData_->damagedMicrophones[MICROPHONES::FRONT] = microFront_();
  headDamageData_->damagedMicrophones[MICROPHONES::REAR] = microRear_();
  headDamageData_->damagedMicrophones[MICROPHONES::LEFT] = microLeft_();
  headDamageData_->damagedMicrophones[MICROPHONES::RIGHT] = microRight_();
  /// Video cameras
  headDamageData_->damagedCameras[CAMERAS::TOP] = cameraTop_();
  headDamageData_->damagedCameras[CAMERAS::BOTTOM] = cameraBottom_();
  /// Infra-Red
  headDamageData_->damagedInfraReds[INFRAREDS::RIGHT] = infraRedRight_();
  headDamageData_->damagedInfraReds[INFRAREDS::LEFT] = infraRedLeft_();
  /// LEDs
  headDamageData_->damagedLEDs[LEDS::RLED0] = rLED0_();
  headDamageData_->damagedLEDs[LEDS::RLED1] = rLED1_();
  headDamageData_->damagedLEDs[LEDS::RLED2] = rLED2_();
  headDamageData_->damagedLEDs[LEDS::RLED3] = rLED3_();
  headDamageData_->damagedLEDs[LEDS::RLED4] = rLED4_();
  headDamageData_->damagedLEDs[LEDS::RLED5] = rLED5_();
  headDamageData_->damagedLEDs[LEDS::RLED6] = rLED6_();
  headDamageData_->damagedLEDs[LEDS::RLED7] = rLED7_();
  headDamageData_->damagedLEDs[LEDS::LLED0] = lLED0_();
  headDamageData_->damagedLEDs[LEDS::LLED1] = lLED1_();
  headDamageData_->damagedLEDs[LEDS::LLED2] = lLED2_();
  headDamageData_->damagedLEDs[LEDS::LLED3] = lLED3_();
  headDamageData_->damagedLEDs[LEDS::LLED4] = lLED4_();
  headDamageData_->damagedLEDs[LEDS::LLED5] = lLED5_();
  headDamageData_->damagedLEDs[LEDS::LLED6] = lLED6_();
  headDamageData_->damagedLEDs[LEDS::LLED7] = lLED7_();
  /// Contact and tactile sensors - Head
  headDamageData_->damagedTactileHeadSensors[TACTILEHEADSENSORS::FRONT] = headTouchFront_();
  headDamageData_->damagedTactileHeadSensors[TACTILEHEADSENSORS::MIDDLE] = headTouchMiddle_();
  headDamageData_->damagedTactileHeadSensors[TACTILEHEADSENSORS::REAR] = headTouchRear_();
}
