#pragma once

#include "Tools/Math/Eigen.hpp"

#include "Data/CycleInfo.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/KickOutput.hpp"
#include "Data/MotionActivation.hpp"

#include "Utils/MotionFile/MotionFilePlayer.hpp"

#include "KickPhase.hpp"

#define TIME_STEP 10


class Motion;

/**
 * @brief execute a dynamic kick that adapts to the current ball position
 */
class Kick : public Module<Kick, Motion>
{
public:
  /**
   * @brief the Kick class
   * @param manager a reference to motion
   */
  Kick(const ModuleManagerInterface& manager);

  void cycle();

private:
  /**
   * @brief check for a kick request and set initial values
   */
  void handleKickRequest();

  /**
   * @brief abort kick and interpolate to ready pose if fallen is detected
   */
  void catchFallen();

  /**
   * @brief filtered gyroscope readings are multiplied by a gain and added to the ankle roll and pitch
   * @param angles the angles with modified ankle roll and pitch
   */
  void applyAnkleController(std::vector<float>& angles);

  /// a reference to the motion activation
  const Dependency<MotionActivation> motionActivation_;

  /// a reference to the motion request
  const Dependency<MotionRequest> motionRequest_;

  /// a reference to the cycle info
  const Dependency<CycleInfo> cycleInfo_;

  /// a reference to the imu sensor data
  const Dependency<IMUSensorData> imuSensorData_;

  /// a reference to the joint sensor data
  const Dependency<JointSensorData> jointSensorData_;

  /// a reference to the kick output
  Production<KickOutput> kickOutput_;

  /// name of the left kick motion file
  const Parameter<std::string> leftMotionFile_;
  /// name of the right kick motion file
  const Parameter<std::string> rightMotionFile_;
  /// the left kick motion
  MotionFilePlayer leftMotion_;
  /// the right kick motion
  MotionFilePlayer rightMotion_;

  KickPhaseHelper kickPhaseHelper_;

  /// the kick type
  KickType kickType_;

  /// current motion phase
  KickPhase::Phase phase_;

  /// whether or not the left foot is kicking
  bool leftKicking_;


  /// duration of motion phases
  const Parameter<unsigned int> toReadyDuration_;
  const Parameter<unsigned int> balanceDuration_;
  const Parameter<unsigned int> liftDuration_;
  const Parameter<unsigned int> swingDuration_;
  const Parameter<unsigned int> retractDuration_;
  const Parameter<unsigned int> extendAndCenterDuration_;
  const Parameter<unsigned int> waitDuration_;
  const Parameter<unsigned int> catchFallenDuration_;

  /// the motion phases
  ToReady toReady_;
  Balance balance_;
  Lift lift_;
  Swing swing_;
  Retract retract_;
  ExtendAndCenter extendAndCenter_;
  Wait wait_;
  CatchFallen catchFallen_;

  /// exponential moving average of angle and gyro readings
  Vector3f angleAccumulator_ = Vector3f::Zero();
  Vector3f gyroAccumulator_ = Vector3f::Zero();
  const Parameter<float> lowPassAlpha_;

  /// ankle controller gains
  const Parameter<float> gainX_;
  const Parameter<float> gainY_;
};
