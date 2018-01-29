#pragma once

#include "Data/BodyRotationData.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/OdometryData.hpp"
#include "Data/WalkingEngineWalkOutput.hpp"
#include "Framework/Module.hpp"

#include "SensorFusion.hpp"


class Motion;

class IMUOdometry : public Module<IMUOdometry, Motion>
{
public:
  /**
   * @brief IMUOdometry initializes members
   * @param manager a reference to motion
   */
  IMUOdometry(const ModuleManagerInterface& manager);
  /**
   * @brief cycle calculates the rotational odometry offset
   */
  void cycle();

private:
  /// filter that estimates body angles using the accelerometer and gyroscope
  SensorFusion sensorFusion_;
  /// the output of the walking engine for translational odometry
  const Dependency<WalkingEngineWalkOutput> walkingEngineWalkOutput_;
  /// the IMU sensor data
  const Dependency<IMUSensorData> imuSensorData_;
  /// the roll pitch yaw angles as equivalent to IMUSensorData::angle
  Production<BodyRotationData> bodyRotationData_;
  /// the accumulated odometry
  Production<OdometryData> odometryData_;
  /// the accumulated local odometry
  Pose accumulatedOdometry_;
};
