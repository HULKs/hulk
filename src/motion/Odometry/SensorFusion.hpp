#pragma once

#include <Framework/Module.hpp>
#include <Tools/Kinematics/KinematicMatrix.h>
#include <Tools/Math/Eigen.hpp>


class SensorFusion
{
public:
  /**
   * @brief: SensorFusion initializes members and loads config params
   * @param module a refernce to the module base
   */
  SensorFusion(const ModuleBase& module);
  /**
   * @brief update updates the sensorFusion with the current gyro and accel measurement
   * @param extGyro the external gyro measurement (omega vector) in rad/s
   * @param extAccel the external acceleration measurement in m/s²
   * @param cycleTime the time a cycle needs run
   */
  void update(const Vector3f& extGyro, const Vector3f& extAccel, const float cycleTime);
  /**
   * @brief setOrientation resets to internal orientation quaternion to a givn roll, pitch, yaw
   * @param orient the orientation to be set in terms of (roll, pitch, yaw)
   */
  void setOrientation(const Vector3d& orient);
  /**
   * @brief getOrientation calculates the current orientation in Euler angles from the internatl state
   * @return the current orientation as (roll, pitch yaw)
   */
  Vector3f getOrientation() const;
  /**
   * @brief getBodyTilt calculates the body2ground rotation as RotationMatrix
   * @return the body2ground rotation as RotationMatrix
   */
  Matrix3f getBodyTilt() const;
  /**
   * @brief getAxisAngles calculates the current orientation in axis angles from the internal state
   * @return the current orientation as axis angles
   */
  Vector3f getAxisAngles() const;

private:
  /**
   * @brief calculateOrientation calculates the (intial) orientation from the external acceleration measurement
   * @param extGyro the external gyro measurement
   * @param extAccel the external acceleration measurement
   */
  void calculateOrientation(const Vector3d& extGyro, const Vector3d& extAccel);
  /**
   * @brief updateOrientationGyro integrates the external gyro measurement for on time step
   * @param extGyro the external gyro measurement
   * @param cycleTime the time a cycle needs run
   */
  void updateOrientationGyro(const Vector3d& extGyro, const float cycleTime);
  /**
   * @brief updateOrientationAccel corrects the orientation with the external acceleration measurement
   * @param extAccel the external accelration measurement
   */
  void updateOrientationAccel(const Vector3d& extAccel);
  /**
   * @brief updateGyroBias updates the internal bias model
   * @param extGyro the external gyro measurement
   * @param extAccel the external acceleration measurement
   */
  void updateGyroBias(const Vector3d& extGyro, const Vector3d& extAccel);
  /**
   * @brief checkSteadyState checks whether the current state can be considered steady
   * @param extGyro the external gyro measurement
   * @param extAccel the external acceleration measurement
   * @return whether in steady state or not
   */
  bool checkSteadyState(const Vector3d& extGyro, const Vector3d& extAccel);

  /// whether the sensorFusion has been initialized
  bool initialized_;
  /// external parameter to reset the orientation for debugging
  Parameter<bool> reset_;
  /// the percentage with which the gravity measurement is respected
  const Parameter<float> accelweight_;
  /// the gravity in m/s²
  const Parameter<float> gravity_;
  /// a factor for the gyro bias
  const Parameter<float> gyro_bias_alpha_;
  /// a threshold for the acceleration to determine steady state
  const Parameter<float> acceleration_threshold_;
  /// a threshold to limit a the minimum gyro change to determine steady state
  const Parameter<float> delta_angular_velocity_threshold_;
  /// a threshold to for thte gyro to determine steady state
  const Parameter<float> angular_velocity_threshold_;

  /// the gyro measurement from the last round
  Vector3d gyro_prev_;
  /// the current gyro bias (substracted from the gyro measurement to get rid of the drift)
  Vector3d gyro_bias_;

  /// the internal state quaternion holding the orientation
  Quaterniond global_to_local_;
};
