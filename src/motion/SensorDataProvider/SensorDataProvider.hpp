#pragma once

#include <array>

#include "Data/BatteryData.hpp"
#include "Data/ButtonData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FSRSensorData.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/RobotKinematics.hpp"
#include "Data/SonarData.hpp"
#include "Definitions/keys.h"
#include "Framework/Module.hpp"


class Motion;

class SensorDataProvider : public Module<SensorDataProvider, Motion>
{
public:
  SensorDataProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  /**
   * @brief fillFSR converts an array of numbers into an FSR sensor struct
   * @param sensor the sensor struct
   * @param data the raw array of sensor data
   */
  void fillFSR(FSRSensorData::Sensor& sensor, const std::array<float, keys::sensor::fsr::FSR_MAX>& data);
  /**
   * @brief fillSonar copies raw sonar data to a clipped value
   * @param clipped the resulting clipped sonar measurement
   * @param raw the raw sonar measurement
   */
  void fillSonar(float& clipped, const float raw);
  Production<FSRSensorData> fsrSensorData_;
  Production<IMUSensorData> imuSensorData_;
  Production<JointSensorData> jointSensorData_;
  Production<BatteryData> batteryData_;
  Production<ButtonData> buttonData_;
  Production<RobotKinematics> robotKinematics_;
  Production<SonarSensorData> sonarSensorData_;
  Production<CycleInfo> cycleInfo_;

  /// the most recently read sensor data
  NaoSensorData sensorData_;
};
