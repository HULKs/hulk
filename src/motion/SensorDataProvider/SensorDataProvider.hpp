#pragma once

#include <array>

#include "BodyDamageProvider/BodyDamageProvider.hpp"
#include "Data/BatteryData.hpp"
#include "Data/ButtonData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FSRSensorData.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/JointCalibrationData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/RobotKinematics.hpp"
#include "Data/SonarData.hpp"
#include "Definitions/keys.h"
#include "Framework/Module.hpp"

class Motion;

class SensorDataProvider : public Module<SensorDataProvider, Motion>
{
public:
  /// the name of this module
  ModuleName name = "SensorDataProvider";
  SensorDataProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  /**
   * @brief fillFSR converts an array of numbers into an FSR sensor struct
   * @param sensor the sensor struct
   * @param data the raw array of sensor data
   */
  void fillFSR(FSRSensorData::Sensor& sensor,
               const std::array<float, keys::sensor::fsr::FSR_MAX>& data);
  /**
   * @brief Checks the sonar echo measurements and sets validity flags accordingly
   * @param sonar the raw sonar distance measurements for all echoes
   * @param input the raw sonar distance measurements
   */
  void setSonarValidity(SonarSensorData& sonar, std::array<float, keys::sensor::SONAR_MAX> input);

  const Dependency<JointCalibrationData> jointCalibrationData_;
  /// used to disable broken sensors
  const Dependency<BodyDamageData> bodyDamageData_;

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
  /**
   * the maximum echo range in meters for the sonar sensors, taken from
   * http://doc.aldebaran.com/2-1/family/nao_dcm/actuator_sensor_names.html#sonars
   */
  const float MAX_SONAR_RANGE = 5;
};
