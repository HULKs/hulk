#pragma once

#include <vector>

#include "Data/CycleInfo.hpp"
#include "Data/HeadMotionOutput.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/MotionRequest.hpp"
#include "Data/RobotKinematics.hpp"
#include "Framework/Module.hpp"
#include "Tools/Kinematics/KinematicMatrix.h"
#include "Tools/Math/Eigen.hpp"


class Motion;

/**
 * @brief The HeadMotion class will control the HeadMotion of the robot. It directly
 * executes the commands, that will be generated from brain to control the direction, the NAO
 * looks at.
 * @author Finn Poppinga
 */
class HeadMotion : public Module<HeadMotion, Motion>
{
public:
  /// the name of this module
  ModuleName name = "HeadMotion";
  /**
   * @brief HeadMotion initializes members
   * @param a reference to motion
   */
  HeadMotion(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks for a new command and sends it to the DCM
   */
  void cycle();

private:
  /**
   * @brief calculateHeadAnglesFromTarget will calculate the head yaw and head pitch from a given
   * (ground) target
   * @param targetPosition the target to look at (on ground, in robot coordinates)
   * @param cam2head the KinematicMatrix of the camera to calculate the joint angles with
   * @return a vector of head yaw and head pitch
   */
  std::vector<float> calculateHeadAnglesFromTarget(const Vector3f& targetPosition,
                                                   const KinematicMatrix& cam2head,
                                                   float yawMax) const;
  /**
   * @brief selectCameraAndAnglesForTarget calculates the requested yaw and pitch angles for both
   * top and bottom camera to look at target using calculateHeadAnglesFromTarget and then selects
   * the angles/camera that require the smallest head motion
   */
  void selectCameraAndAnglesForTarget(const Vector3f& targetPosition);
  /**
   * @brief calculateJointAnglesFromRequest generates the joint angles from the requested angle
   */
  void calculateJointAnglesFromRequest();
  /**
   * @brief resetFilters() resets the filtered values to avoid harmfull accumulation (e.g. after
   * fallen)
   */
  void resetFilters();
  /**
   * @brief filterSensorData() filters the sensor data that is used for the head motion in order to
   * achieve a smooth motion.
   */
  void filterSensorData();

  /// the maximum allowable yaw velocity [rad/s]
  const Parameter<float> maxYawVelocity_;
  /// the maximum allowable pitch velocity [rad/s]
  const Parameter<float> maxPitchVelocity_;
  /// the maximum pitch when abs(yaw) > yawThreshold
  const Parameter<float> outerPitchMax_;
  /// the maximum pitch when yaw = 0
  const Parameter<float> innerPitchMax_;
  /// the yaw threshold
  const Parameter<float> yawThreshold_;
  /// the low pass ratio used to filter the gyroscope
  const Parameter<float> lowPassAlphaGyro_;

  /// a reference to the motion request
  const Dependency<MotionRequest> motionRequest_;
  /// a reference to the motion activation
  const Dependency<MotionActivation> motionActivation_;
  /// a reference to the motion cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// a reference to the joint sensor data
  const Dependency<JointSensorData> jointSensorData_;
  /// a reference to the robot kinematics
  const Dependency<RobotKinematics> robotKinematics_;
  /// a reference to the imu sensor data used to figure out the torso rotation speed relative to
  /// ground
  const Dependency<IMUSensorData> imuSensorData_;
  /// a reference to the head motion output
  Production<HeadMotionOutput> headMotionOutput_;

  /// the filtered angualr velocity of the torso with respect to the ground
  float filteredTorsoYawVelocity_;

  /// the head yaw
  float requestedHeadYaw_;
  /// the head pitch
  float requestedHeadPitch_;
  /// the maximum allowed head yaw velocity
  float requestedHeadYawVelocity_;
  /// the maximum allowed head pitch velocity
  float requestedHeadPitchVelocity_;
  /// true if the requested velocity is to be achieved relative to the ground
  bool useEffectiveYawVelocity_;
  /// whether the head motion module was in control of the joint angles in the last cycle
  bool wasActive_;
  /// whether the head was at the target in the last cycle
  bool wasAtTarget_;
  /// local state when head reached target
  TimePoint timeWhenReachedTarget_;
  /// the head joint angles that are currently active
  std::vector<float> jointAngles_;
};
