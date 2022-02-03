#include "Motion/SensorDataProvider/SensorDataProvider.hpp"
#include "Framework/Log/Log.hpp"
#include "Tools/Chronometer.hpp"
#include <chrono>

SensorDataProvider::SensorDataProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , jointCalibrationData_(*this)
  , bodyDamageData_(*this)
  , fsrSensorData_(*this)
  , imuSensorData_(*this)
  , jointSensorData_(*this)
  , buttonData_(*this)
  , sonarSensorData_(*this)
  , cycleInfo_(*this)
{
}

void SensorDataProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  robotInterface().produceSensorData(*cycleInfo_, *fsrSensorData_, *imuSensorData_,
                                     *jointSensorData_, *buttonData_, *sonarSensorData_);

  // This needs to be the first call to debug in the ModuleManager per cycle
  debug().setUpdateTime(cycleInfo_->startTime);

  for (std::size_t i{0}; i < static_cast<std::size_t>(Joints::MAX); i++)
  {
    const auto joint{static_cast<Joints>(i)};
    jointSensorData_->angles[joint] -= jointCalibrationData_->calibrationOffsets[joint];
  }
}
