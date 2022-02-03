#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FSRSensorData.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/StandUpResult.hpp"
#include "Framework/Module.hpp"
#include "Tools/Math/Eigen.hpp"
#include <array>


class Motion;

class BodyPoseEstimation : public Module<BodyPoseEstimation, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"BodyPoseEstimation"};
  /**
   * @brief BodyPoseEstimation initializes members and resets buffers
   * @param manager a reference to motion
   */
  explicit BodyPoseEstimation(const ModuleManagerInterface& manager);
  /**
   * @brief cycle estimates some information about the pose of the body
   */
  void cycle() override;

private:
  const Dependency<ActionCommand> actionCommand_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<StandUpResult> standUpResult_;
  const Dependency<IMUSensorData> imuSensorData_;
  const Dependency<FSRSensorData> fsrSensorData_;
  /// a reference to the motion activation of last cycle
  const Reference<MotionActivation> motionActivation_;

  /// the resulting pose estimation
  Production<BodyPose> bodyPose_;

  /// the upper threshold for accelerometer.x for the robot to be considered upright
  const Parameter<float> uprightUpThreshX_;
  /// the upper threshold for accelerometer.y for the robot to be considered upright
  const Parameter<float> uprightUpThreshY_;
  /// the lower threshold for accelerometer.z for the robot to be considered upright
  const Parameter<float> uprightLoThreshZ_;
  /// the minimum pressure assumed per fsr
  const Parameter<float> minFsrPressure_;
  /// the maximum pressure assumed per fsr
  const Parameter<float> maxFsrPressure_;
  /// weighting for outer fsrs
  const Parameter<float> outerFsrWeight_;
  /// weighting for inner fsrs
  const Parameter<float> innerFsrWeight_;
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
  /// the maximum gyro norm to bo not wonky
  const Parameter<float> maxGyroNormNotWonky_;

  /// the number of weights to keep in the buffer
  static constexpr std::size_t weightBufferSize__{10};
  /// whether the robot is currently fallen
  bool fallen_{false};
  /// the fall direction
  BodyPose::FallDirection fallDirection_{BodyPose::FallDirection::NOT_FALLING};
  /// the filtered norm of the gyro vector
  float filteredGyroNorm_{0.f};
  /// the last state of the body motion
  ActionCommand::Body::MotionType lastBodyMotionType_{ActionCommand::Body::MotionType::DEAD};
  /// the time at which the robot started to fall down
  Clock::time_point timeWhenFallen_;
  /// the time at which the robot last had contact with its feet
  Clock::time_point timeOfLastFootContact_;
  /// a buffer of the last few weights on the FSRs
  std::array<float, weightBufferSize__> weightBuffer_{};
  /// the index at which to write the next value in the weight buffer
  std::size_t weightBufferPosition_{0};
  /// the sum of all values in the FSR weight buffer
  float weightBufferSum_{0.f};
  /// weights of the individual fsrs
  FSRsArray<float> weights_{};
  /// highest pressure measured up to now per fsr
  FSRsArray<float> highestPressure_{};
  /// the side of support (positive if left support)
  float lastSupportSide_{0.f};
  /// minimum angle around X
  float tempXmin_{0.f};
  /// maximum angle around X
  float tempXmax_{0.f};
  /// minimum angle around Y
  float tempYmin_{0.f};
  /// maximum angle around Y
  float tempYmax_{0.f};
  /// accumulator for angle readings
  Vector2f angleAccumulator_{Vector2f::Zero()};
  /// accumulator for gyro readings
  Vector3f gyroAccumulator_{Vector3f::Zero()};

  /**
   * @brief detectFalling detects the direction in which the robot is falling
   */
  void detectFalling();
  /**
   * @brief detectWonky detects whether the robot is wonky
   */
  void detectWonky();
  /**
   * @brief determineApproxUpright determines whether the robot is approximately upright
   */
  void determineApproxUpright();
  /**
   * @brief determineFootContact determines whether the robot feet touch the ground
   */
  void determineFootContact();
  /**
   * @brief determineSupportFoot determines which of the feet is the support foot and whether the
   * support foot has changed
   */
  void determineSupportFoot();
  /**
   * @brief printAngleExtremes can be used to determine the angles that occur during normal
   * operation
   */
  void sendAngleExtremes();
};
