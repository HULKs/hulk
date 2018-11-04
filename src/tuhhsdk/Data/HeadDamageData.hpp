#pragma once

#include "Framework/DataType.hpp"
#include "Modules/NaoProvider.h"
#include <array>

class HeadDamageData : public DataType<HeadDamageData>
{
public:
  /// the name of this DataType
  DataTypeName name = "HeadDamageData";
  /// broken state of the hardware
  std::array<bool, SPEAKERS::SPEAKERS_MAX> damagedSpeakers;
  std::array<bool, MICROPHONES::MICROPHONES_MAX> damagedMicrophones;
  std::array<bool, CAMERAS::CAMERAS_MAX> damagedCameras;
  std::array<bool, INFRAREDS::INFRARED_MAX> damagedInfraReds;
  std::array<bool, LEDS::LEDS_MAX> damagedLEDs;
  std::array<bool, TACTILEHEADSENSORS::TACTILEHEADSENSORS_MAX> damagedTactileHeadSensors;

  void reset() {}

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["damagedSpeakers"] << damagedSpeakers;
    value["damagedMicrophones"] << damagedMicrophones;
    value["damagedCameras"] << damagedCameras;
    value["damagedInfraReds"] << damagedInfraReds;
    value["damagedLEDs"] << damagedLEDs;
    value["damagedTactileHeadSensors"] << damagedTactileHeadSensors;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["damagedSpeakers"] >> damagedSpeakers;
    value["damagedMicrophones"] >> damagedMicrophones;
    value["damagedCameras"] >> damagedCameras;
    value["damagedInfraReds"] >> damagedInfraReds;
    value["damagedLEDs"] >> damagedLEDs;
    value["damagedTactileHeadSensors"] >> damagedTactileHeadSensors;
  }
};
