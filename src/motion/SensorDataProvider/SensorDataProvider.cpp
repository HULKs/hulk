#include <algorithm>
#include <cassert>

#include "Tools/Chronometer.hpp"
#include "Tools/Kinematics/Com.h"
#include "Tools/Kinematics/ForwardKinematics.h"

#include "SensorDataProvider.hpp"


SensorDataProvider::SensorDataProvider(const ModuleManagerInterface& manager)
  : Module(manager, "SensorDataProvider")
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

  cycleInfo_->cycleTime = 0.01;
  cycleInfo_->startTime = sensorData_.time;

  fillFSR(fsrSensorData_->left, sensorData_.fsrLeft);
  fillFSR(fsrSensorData_->right, sensorData_.fsrRight);
  debug().update(mount_ + ".FSRSensorData", *fsrSensorData_);

  imuSensorData_->accelerometer =
      Vector3f(sensorData_.imu[keys::sensor::IMU_ACC_X], sensorData_.imu[keys::sensor::IMU_ACC_Y], sensorData_.imu[keys::sensor::IMU_ACC_Z]);
  imuSensorData_->angle =
      Vector3f(sensorData_.imu[keys::sensor::IMU_ANGLE_X], sensorData_.imu[keys::sensor::IMU_ANGLE_Y], sensorData_.imu[keys::sensor::IMU_ANGLE_Z]);
  imuSensorData_->gyroscope =
      Vector3f(sensorData_.imu[keys::sensor::IMU_GYR_X], sensorData_.imu[keys::sensor::IMU_GYR_Y], sensorData_.imu[keys::sensor::IMU_GYR_Z]);
  debug().update(mount_ + ".IMUSensorData", *imuSensorData_);

  jointSensorData_->angles = sensorData_.jointSensor;
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

  fillSonar(sonarSensorData_->sonarLeft, sensorData_.sonar[keys::sensor::SONAR_LEFT_SENSOR_0]);
  fillSonar(sonarSensorData_->sonarRight, sensorData_.sonar[keys::sensor::SONAR_RIGHT_SENSOR_0]);
  debug().update(mount_ + ".SonarSensorData", *sonarSensorData_);

  const std::vector<KinematicMatrix> kinematicMatrices = ForwardKinematics::getBody(jointSensorData_->getBodyAngles(), imuSensorData_->angle);
  assert(kinematicMatrices.size() == robotKinematics_->matrices.size());
  std::copy(kinematicMatrices.begin(), kinematicMatrices.end(), robotKinematics_->matrices.begin());
  robotKinematics_->com = Com::getComBody(kinematicMatrices);
}

void SensorDataProvider::fillFSR(FSRSensorData::Sensor& sensor, const std::array<float, keys::sensor::fsr::FSR_MAX>& data)
{
  sensor.frontLeft = data[keys::sensor::fsr::FSR_FRONT_LEFT];
  sensor.frontRight = data[keys::sensor::fsr::FSR_FRONT_RIGHT];
  sensor.rearLeft = data[keys::sensor::fsr::FSR_REAR_LEFT];
  sensor.rearRight = data[keys::sensor::fsr::FSR_REAR_RIGHT];
  sensor.totalWeight = data[keys::sensor::fsr::FSR_TOTAL_WEIGHT];
  sensor.cop.x() = data[keys::sensor::fsr::FSR_COP_X];
  sensor.cop.y() = data[keys::sensor::fsr::FSR_COP_Y];
}

void SensorDataProvider::fillSonar(float& clipped, const float raw)
{
  // This is old Blackboard functionality which is not removed for safety reasons.
  if (raw > 0.f && raw < 2.f)
  {
    clipped = raw;
  }
  else
  {
    clipped = -1.f;
  }
}
