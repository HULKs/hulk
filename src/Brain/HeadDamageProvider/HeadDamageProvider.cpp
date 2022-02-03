#include "Brain/HeadDamageProvider/HeadDamageProvider.hpp"

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
  , ledLeftEar_(*this, "ledLeftEar", [this] { damageStateChanged_ = true; })
  , ledRightEar_(*this, "ledRightEar", [this] { damageStateChanged_ = true; })
  , ledLeftEye_(*this, "ledLeftEye", [this] { damageStateChanged_ = true; })
  , ledRightEye_(*this, "ledRightEye", [this] { damageStateChanged_ = true; })
  , ledSkull_(*this, "ledSkull", [this] { damageStateChanged_ = true; })
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
  headDamageData_->damagedSpeakers[Speakers::LEFT] = leftSpeaker_();
  headDamageData_->damagedSpeakers[Speakers::RIGHT] = rightSpeaker_();
  /// Microphones
  headDamageData_->damagedMicrophones[Microphones::FRONT] = microFront_();
  headDamageData_->damagedMicrophones[Microphones::REAR] = microRear_();
  headDamageData_->damagedMicrophones[Microphones::LEFT] = microLeft_();
  headDamageData_->damagedMicrophones[Microphones::RIGHT] = microRight_();
  /// Video cameras
  headDamageData_->damagedCameras[Cameras::TOP] = cameraTop_();
  headDamageData_->damagedCameras[Cameras::BOTTOM] = cameraBottom_();
  /// Infra-Red
  headDamageData_->damagedInfraReds[Infrareds::RIGHT] = infraRedRight_();
  headDamageData_->damagedInfraReds[Infrareds::LEFT] = infraRedLeft_();
  /// LEDs
  headDamageData_->damagedLEDs[HeadLEDs::L_EAR] = ledLeftEar_();
  headDamageData_->damagedLEDs[HeadLEDs::R_EAR] = ledRightEar_();
  headDamageData_->damagedLEDs[HeadLEDs::L_EYE] = ledLeftEye_();
  headDamageData_->damagedLEDs[HeadLEDs::R_EYE] = ledRightEye_();
  headDamageData_->damagedLEDs[HeadLEDs::SKULL] = ledSkull_();
  /// Contact and tactile sensors - HEAD
  headDamageData_->damagedSwitches[HeadSwitches::HEAD_FRONT] = headTouchFront_();
  headDamageData_->damagedSwitches[HeadSwitches::HEAD_MIDDLE] = headTouchMiddle_();
  headDamageData_->damagedSwitches[HeadSwitches::HEAD_REAR] = headTouchRear_();
}
