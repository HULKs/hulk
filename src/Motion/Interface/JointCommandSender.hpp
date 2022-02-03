#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/BodyDamageData.hpp"
#include "Data/FallManagerOutput.hpp"
#include "Data/HeadMotionOutput.hpp"
#include "Data/JointCalibrationData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/JumpOutput.hpp"
#include "Data/KickOutput.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/MotionState.hpp"
#include "Data/PointOutput.hpp"
#include "Data/Poses.hpp"
#include "Data/PuppetMotionOutput.hpp"
#include "Data/SitDownOutput.hpp"
#include "Data/SitUpOutput.hpp"
#include "Data/StandUpOutput.hpp"
#include "Data/WalkGeneratorOutput.hpp"
#include "Framework/Module.hpp"

class Motion;

class JointCommandSender : public Module<JointCommandSender, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"JointCommandSender"};
  /**
   * @brief JointCommandSender initializes members
   * @param manager a reference to motion
   */
  explicit JointCommandSender(const ModuleManagerInterface& manager);
  /**
   * @brief cycle uses some joint commands from a motion module and sends them to the DCM
   */
  void cycle() override;

private:
  const Dependency<MotionActivation> motionActivation_;
  const Dependency<FallManagerOutput> fallManagerOutput_;
  const Dependency<HeadMotionOutput> headMotionOutput_;
  const Dependency<JumpOutput> jumpOutput_;
  const Dependency<KickOutput> kickOutput_;
  const Dependency<PointOutput> pointOutput_;
  const Dependency<StandUpOutput> standUpOutput_;
  const Dependency<SitDownOutput> sitDownOutput_;
  const Dependency<SitUpOutput> sitUpOutput_;
  const Dependency<WalkGeneratorOutput> walkGeneratorOutput_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<JointCalibrationData> jointCalibrationData_;
  const Dependency<BodyDamageData> bodyDamageData_;
  const Dependency<PuppetMotionOutput> puppetMotionOutput_;
  const Dependency<Poses> poses_;

  Production<MotionState> motionState_;

  /// the joint angles when interpolation started
  JointsArray<float> startInterpolationAngles_{};
};
