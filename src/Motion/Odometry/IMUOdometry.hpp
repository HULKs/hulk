#pragma once

#include "Data/BodyPose.hpp"
#include "Data/BodyRotationData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/MotionState.hpp"
#include "Data/OdometryData.hpp"
#include "Data/RobotKinematics.hpp"
#include "Framework/Module.hpp"
#include "Motion/Odometry/SensorFusion.hpp"
#include "Tools/Math/MovingAverage.hpp"


class Motion;

class IMUOdometry : public Module<IMUOdometry, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"IMUOdometry"};
  /**
   * @brief IMUOdometry initializes members
   * @param manager a reference to motion
   */
  explicit IMUOdometry(const ModuleManagerInterface& manager);
  /**
   * @brief cycle calculates the rotational odometry offset
   */
  void cycle() override;
  /**
   * @brief detectOrientationDrift detects if the gyroscope keeps changing while the robot is still
   * and emits a warning
   */
  void detectOrientationDrift();

private:
  /// filter that estimates body angles using the accelerometer and gyroscope
  SensorFusion sensorFusion_;
  const Dependency<RobotKinematics> robotKinematics_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<IMUSensorData> imuSensorData_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<MotionState> motionState_;

  /// the roll pitch yaw angles as equivalent to IMUSensorData::angle
  Production<BodyRotationData> bodyRotationData_;
  /// the accumulated odometry
  Production<OdometryData> odometryData_;

  /// the accumulated local odometry
  Pose accumulatedOdometry_;
  /// the cycles since we last checked for an orientation drift
  unsigned int cycleSinceLastDriftCheck_{0};
  /// value of last orientation
  float lastOrientation_{0.f};
  /// average of the last 256 orientation deltas
  SimpleArrayMovingAverage<float, float, 256> deltaOrientationAverage_;
};
