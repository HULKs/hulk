#include <algorithm>
#include <cassert>

#include "BodyDamageProvider/BodyDamageProvider.hpp"
#include "Definitions/keys.h"
#include "Modules/NaoProvider.h"
#include "Tools/Chronometer.hpp"
#include "Tools/Kinematics/Com.h"
#include "Tools/Kinematics/ForwardKinematics.h"

#include "SensorDataProvider.hpp"


SensorDataProvider::SensorDataProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , jointCalibrationData_(*this)
  , bodyDamageData_(*this)
  , fsrSensorData_(*this)
  , imuSensorData_(*this)
  , jointSensorData_(*this)
  , batteryData_(*this)
  , buttonData_(*this)
  , robotKinematics_(*this)
  , sonarSensorData_(*this)
  , cycleInfo_(*this)
{
}

void SensorDataProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  robotInterface().waitAndReadSensorData(sensorData_);

  // This needs to be the first call to debug in the ModuleManager per cycle
  debug().setUpdateTime(sensorData_.time);

  cycleInfo_->cycleTime = 0.01;
  cycleInfo_->startTime = sensorData_.time;

  fillFSR(fsrSensorData_->left, sensorData_.fsrLeft);
  fillFSR(fsrSensorData_->right, sensorData_.fsrRight);
  debug().update(mount_ + ".FSRSensorData", *fsrSensorData_);

  imuSensorData_->accelerometer =
      Vector3f(sensorData_.imu[keys::sensor::IMU_ACC_X], sensorData_.imu[keys::sensor::IMU_ACC_Y],
               sensorData_.imu[keys::sensor::IMU_ACC_Z]);
  imuSensorData_->angle = Vector3f(sensorData_.imu[keys::sensor::IMU_ANGLE_X],
                                   sensorData_.imu[keys::sensor::IMU_ANGLE_Y],
                                   sensorData_.imu[keys::sensor::IMU_ANGLE_Z]);
  imuSensorData_->gyroscope =
      Vector3f(sensorData_.imu[keys::sensor::IMU_GYR_X], sensorData_.imu[keys::sensor::IMU_GYR_Y],
               sensorData_.imu[keys::sensor::IMU_GYR_Z]);
  debug().update(mount_ + ".IMUSensorData", *imuSensorData_);

  for (unsigned int i = 0; i < sensorData_.jointSensor.size(); i++)
  {
    jointSensorData_->angles[i] =
        sensorData_.jointSensor[i] - jointCalibrationData_->calibrationOffsets[i];
  }
  jointSensorData_->currents = sensorData_.jointCurrent;
  jointSensorData_->temperatures = sensorData_.jointTemperature;
  jointSensorData_->status = sensorData_.jointStatus;
  debug().update(mount_ + ".JointSensorData", *jointSensorData_);

  const std::array<float, keys::sensor::BATTERY_MAX>& battery = sensorData_.battery;
  batteryData_->charge = battery[keys::sensor::BATTERY_CHARGE];
  batteryData_->current = battery[keys::sensor::BATTERY_CURRENT];
  batteryData_->temperature = battery[keys::sensor::BATTERY_TEMPERATURE];
  batteryData_->status = battery[keys::sensor::BATTERY_STATUS];
  debug().update(mount_ + ".BatteryData", *batteryData_);

  buttonData_->buttons = sensorData_.switches;
  const std::vector<callback_event>& callbackEvents = sensorData_.buttonCallbackList;
  for (auto cb : callbackEvents)
  {
    if (cb == CE_CHESTBUTTON_SIMPLE)
    {
      buttonData_->lastChestButtonSinglePress = cycleInfo_->startTime;
    }
    else if (cb == CE_CHESTBUTTON_DOUBLE)
    {
      buttonData_->lastChestButtonDoublePress = cycleInfo_->startTime;
    }
  }
  sensorData_.buttonCallbackList.clear();
  debug().update(mount_ + ".ButtonData", *buttonData_);

  sonarSensorData_->data = sensorData_.sonar;
  setSonarValidity(*sonarSensorData_, sensorData_.sonar);
  
  const std::vector<KinematicMatrix> kinematicMatrices =
      ForwardKinematics::getBody(jointSensorData_->getBodyAngles(), imuSensorData_->angle);
  assert(kinematicMatrices.size() == robotKinematics_->matrices.size());
  std::copy(kinematicMatrices.begin(), kinematicMatrices.end(), robotKinematics_->matrices.begin());
  robotKinematics_->com = Com::getComBody(kinematicMatrices);

  jointSensorData_->valid = true;
  imuSensorData_->valid = true;
  fsrSensorData_->valid = true;
  buttonData_->valid = true;
}

void SensorDataProvider::fillFSR(FSRSensorData::Sensor& sensor,
                                 const std::array<float, keys::sensor::fsr::FSR_MAX>& data)
{
  sensor.frontLeft = data[keys::sensor::fsr::FSR_FRONT_LEFT];
  sensor.frontRight = data[keys::sensor::fsr::FSR_FRONT_RIGHT];
  sensor.rearLeft = data[keys::sensor::fsr::FSR_REAR_LEFT];
  sensor.rearRight = data[keys::sensor::fsr::FSR_REAR_RIGHT];
  sensor.totalWeight = data[keys::sensor::fsr::FSR_TOTAL_WEIGHT];
  sensor.cop.x() = data[keys::sensor::fsr::FSR_COP_X];
  sensor.cop.y() = data[keys::sensor::fsr::FSR_COP_Y];
}

void SensorDataProvider::setSonarValidity(SonarSensorData& sonar,
  std::array<float, keys::sensor::SONAR_MAX> input)
{
  // set the unused fields to valid
  sonar.valid[keys::sensor::SONAR_ACTUATOR] = sonar.valid[keys::sensor::SONAR_SENSOR] = true;

  // Check if sonar sensors are damaged
  if (!bodyDamageData_->damagedSonars[SONARS::LEFT])
  {
    // Set validity for left echoes
    for (int i = keys::sensor::SONAR_LEFT_SENSOR_0; i <= keys::sensor::SONAR_LEFT_SENSOR_9; i++)
    {
      // A value <= 0 less means error, >= MAX_DETECTION_RANGE means no echo. Source:
      // http://doc.aldebaran.com/2-1/family/nao_dcm/actuator_sensor_names.html#term-us-sensors-m
      sonar.valid[i] = input[i] > 0 && input[i] < MAX_SONAR_RANGE;
    }
  }
  if (!bodyDamageData_->damagedSonars[SONARS::RIGHT])
  {
    // Set validity for right echoes
    for (int i = keys::sensor::SONAR_RIGHT_SENSOR_0; i <= keys::sensor::SONAR_RIGHT_SENSOR_9; i++)
    {
      // A value <= 0 less means error, >= MAX_DETECTION_RANGE means no echo. Source:
      // http://doc.aldebaran.com/2-1/family/nao_dcm/actuator_sensor_names.html#term-us-sensors-m
      sonar.valid[i] = input[i] > 0 && input[i] < MAX_SONAR_RANGE;
    }
  }
}
