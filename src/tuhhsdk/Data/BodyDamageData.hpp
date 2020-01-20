#pragma once

#include "Framework/DataType.hpp"
#include "Modules/NaoProvider.h"
#include <array>

class BodyDamageData : public DataType<BodyDamageData>
{
public:
  /// the name of this DataType
  DataTypeName name = "BodyDamageData";
  /// broken state of the hardware
  std::array<bool, JOINTS::JOINTS_MAX> damagedJoints;
  std::array<bool, FSRS::FSR_MAX> damagedFSRs;
  std::array<bool, IMU::IMU_MAX> damagedIMU;
  std::array<bool, SONARS::SONARS_MAX> damagedSonars;
  std::array<bool, BUTTONS::BUTTONS_MAX> damagedButtons;
  std::array<bool, TACTILEHANDSENSORS::TACTILEHANDSENSORS_MAX> damagedTactileHandSensors;
  std::array<bool, BUMPERS::BUMPERS_MAX> damagedBumpers;

  void reset() override {}

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["damagedJoints"] << damagedJoints;
    value["damagedFSRs"] << damagedFSRs;
    value["damagedIMU"] << damagedIMU;
    value["damagedSonars"] << damagedSonars;
    value["damagedTactileHandSensors"] << damagedTactileHandSensors;
    value["damagedButtons"] << damagedButtons;
    value["damagedBumpers"] << damagedBumpers;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["damagedJoints"] >> damagedJoints;
    value["damagedFSRs"] >> damagedFSRs;
    value["damagedIMU"] >> damagedIMU;
    value["damagedSonars"] >> damagedSonars;
    value["damagedTactileHandSensors"] >> damagedTactileHandSensors;
    value["damagedButtons"] >> damagedButtons;
    value["damagedBumpers"] >> damagedBumpers;
  }
};
