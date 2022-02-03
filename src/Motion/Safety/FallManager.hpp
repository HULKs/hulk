#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FallManagerOutput.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/Poses.hpp"
#include "Framework/Module.hpp"
#include "Motion/Utils/Interpolator/Interpolator.hpp"
#include "Motion/Utils/MotionFile/MotionFilePlayer.hpp"


class Motion;

class FallManager : public Module<FallManager, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"FallManager"};
  /**
   * @brief FallManager initializes members and loads motion files
   * @param manager a reference to motion
   */
  explicit FallManager(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks if the robot is falling and initializes a motion to prevent it
   */
  void cycle() override;

private:
  const Dependency<ActionCommand> actionCommand_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<JointSensorData> jointSensorData_;
  /// a reference to the motion activation of last cycle
  const Reference<MotionActivation> motionActivation_;
  const Dependency<Poses> poses_;

  Production<FallManagerOutput> fallManagerOutput_;

  /// the name of the motion file for kneeing
  const Parameter<std::string> kneeDownMotionFile_;
  /// whether the FallManager is enabled to do something
  const Parameter<bool> enabled_;
  /// Head joint rapid reach stiffness
  const Parameter<float> rapidReachStiffness_;
  /// the catch front interpolation duration
  const Parameter<Clock::duration> catchFrontDuration_;
  /// the catch front hip pitch
  Parameter<float> catchFrontHipPitch_;
  /// Head yaw stiffness increase threshold
  Parameter<float> headYawStiffnessThresh_;
  /// Head pitch stiffness increase threshold
  Parameter<float> headPitchStiffnessThresh_;

  /// whether the fall manager should initiate a fall preventing motion
  bool hot_{false};
  /// interpolator for catch front
  Interpolator<Clock::duration, static_cast<std::size_t>(Joints::MAX)> catchFrontInterpolator_;
  /// motion file for kneeing
  MotionFilePlayer kneeDown_;
  /// the last fall manager output
  JointsArray<float> lastAngles_{};

  /**
   * @brief prepareFalling is executed when falling is detected
   * @param fallDirection the falling direction tendency
   */
  void prepareFalling(BodyPose::FallDirection fallDirection);
  /**
   * @brief stiffnessController will adjust head joint stiffnesses to rapidly reach destination and
   * relax once reached.
   */
  void stiffnessController();
};
