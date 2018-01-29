#pragma once

#include <array>

#include "Tools/Math/Eigen.hpp"

#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FSRSensorData.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/MotionState.hpp"
#include "Data/StandUpResult.hpp"
#include "Framework/Module.hpp"


class Motion;

class BodyPoseEstimation : public Module<BodyPoseEstimation, Motion>
{
public:
  /**
   * @brief BodyPoseEstimation initializes members and resets buffers
   * @param manager a reference to motion
   */
  BodyPoseEstimation(const ModuleManagerInterface& manager);
  /**
   * @brief cycle estimates some information about the pose of the body
   */
  void cycle();

private:
  using BodyMotion = MotionRequest::BodyMotion;
  /**
   * @brief detectFalling detects the direction in which the robot is falling
   */
  void detectFalling();
  /**
   * @brief determineFootContact determines whether the robot feet touch the ground
   */
  void determineFootContact();
  /**
   * @brief printAngleExtremes can be used to determine the angles that occur during normal operation
   */
  void sendAngleExtremes();
  /// the number of weights to keep in the buffer
  static constexpr std::size_t weightBufferSize_ = 10;
  /// the maximum value of the FSR weight if not touching the ground
  const Parameter<float> weightThreshold_;
  /// flag to enable gyro classification for lifted robots
  const Parameter<bool> classifyHighByGyro_;
  /// if the gyro norm is greater than this, the robot is considered to be moved
  const Parameter<float> movingGyroNormThreshold_;
  /// the lowest x angle that may occur before falling
  const Parameter<float> xMin_;
  /// the highest x angle that may occur before falling
  const Parameter<float> xMax_;
  /// the lowest y angle that may occur before falling
  const Parameter<float> yMin_;
  /// the highest y angle that may occur before falling
  const Parameter<float> yMax_;
  /// the lowest x angular velocity that may occur before falling
  const Parameter<float> xdMin_;
  /// the highest x angular velocity that may occur before falling
  const Parameter<float> xdMax_;
  /// the lowest y angular velocity that may occur before falling
  const Parameter<float> ydMin_;
  /// the highest y angular velocity that may occur before falling
  const Parameter<float> ydMax_;
  /// a reference to the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// a reference to the stand up result
  const Dependency<StandUpResult> standUpResult_;
  /// a reference to the IMU sensor data
  const Dependency<IMUSensorData> imuSensorData_;
  /// a reference to the FSR sensor data
  const Dependency<FSRSensorData> fsrSensorData_;
  /// areference to the motion state
  const Reference<MotionState> motionState_;
  /// the resulting pose estimation
  Production<BodyPose> bodyPose_;
  /// whether the robot is currently fallen
  bool fallen_;
  /// the filtered norm of the gyro vector
  float filteredGyroNorm_;
  /// the last state of the body motion
  BodyMotion lastBodyMotionState_;
  /// the time at which the robot started to fall down
  TimePoint timeWhenFallen_;
  /// the time at which the robot last had contact with its feet
  TimePoint timeOfLastFootContact_;
  /// a buffer of the last few weights on the FSRs
  std::array<float, weightBufferSize_> weightBuffer_;
  /// the index at which to write the next value in the weight buffer
  std::size_t weightBufferPosition_;
  /// the sum of all values in the FSR weight buffer
  float weightBufferSum_;
  /// minimum angle around X
  float tempXmin_ = 0;
  /// maximum angle around X
  float tempXmax_ = 0;
  /// minimum angle around Y
  float tempYmin_ = 0;
  /// maximum angle around Y
  float tempYmax_ = 0;
  /// accumulator for angle readings
  Vector3f angleAccumulator_;
  /// accumulator for gyro readings
  Vector3f gyroAccumulator_;
};
