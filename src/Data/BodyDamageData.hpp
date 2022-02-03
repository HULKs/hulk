#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Definitions.hpp"
#include <map>

class BodyDamageData : public DataType<BodyDamageData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"BodyDamageData"};
  /// broken state of the hardware
  JointsArray<bool> damagedJoints{};
  FSRsArray<bool> damagedFSRs{};
  bool damagedGyroscope{};
  bool damagedAccelerometer{};
  SonarsArray<bool> damagedSonars{};
  BodySwitchesArray<bool> damagedSwitches{};
  BodyLEDsArray<bool> damagedLEDs{};

  void reset() override {}

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["damagedJoints"] << damagedJoints;
    value["damagedFSRs"] << damagedFSRs;
    value["damagedGyroscope"] << damagedGyroscope;
    value["damagedAccelerometer"] << damagedAccelerometer;
    value["damagedSonars"] << damagedSonars;
    value["damagedSwitches"] << damagedSwitches;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["damagedJoints"] >> damagedJoints;
    value["damagedFSRs"] >> damagedFSRs;
    value["damagedGyroscope"] >> damagedGyroscope;
    value["damagedAccelerometer"] >> damagedAccelerometer;
    value["damagedSonars"] >> damagedSonars;
    value["damagedSwitches"] >> damagedSwitches;
  }
};
