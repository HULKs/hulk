#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Definitions.hpp"
#include <array>

class HeadDamageData : public DataType<HeadDamageData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"HeadDamageData"};
  /// broken state of the hardware
  SpeakersArray<bool> damagedSpeakers{};
  MicrophonesArray<bool> damagedMicrophones{};
  CamerasArray<bool> damagedCameras{};
  InfraredsArray<bool> damagedInfraReds{};
  HeadLEDsArray<bool> damagedLEDs{};
  HeadSwitchesArray<bool> damagedSwitches{};

  void reset() override {}

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["damagedSpeakers"] << damagedSpeakers;
    value["damagedMicrophones"] << damagedMicrophones;
    value["damagedCameras"] << damagedCameras;
    value["damagedInfraReds"] << damagedInfraReds;
    value["damagedLEDs"] << damagedLEDs;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["damagedSpeakers"] >> damagedSpeakers;
    value["damagedMicrophones"] >> damagedMicrophones;
    value["damagedCameras"] >> damagedCameras;
    value["damagedInfraReds"] >> damagedInfraReds;
    value["damagedLEDs"] >> damagedLEDs;
  }
};
