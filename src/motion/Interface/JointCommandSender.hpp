#pragma once

#include "Data/BodyDamageData.hpp"
#include "Data/FallManagerOutput.hpp"
#include "Data/HeadMotionOutput.hpp"
#include "Data/JointCalibrationData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/JumpOutput.hpp"
#include "Data/KickOutput.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/MotionRequest.hpp"
#include "Data/MotionState.hpp"
#include "Data/PointOutput.hpp"
#include "Data/PoserOutput.hpp"
#include "Data/SitDownOutput.hpp"
#include "Data/SitUpOutput.hpp"
#include "Data/StandUpOutput.hpp"
#include "Data/WalkingEngineStandOutput.hpp"
#include "Data/WalkingEngineWalkOutput.hpp"
#include "Framework/Module.hpp"

class Motion;

class JointCommandSender : public Module<JointCommandSender, Motion>
{
public:
  /// the name of this module
  ModuleName name = "JointCommandSender";
  /**
   * @brief JointCommandSender initializes members
   * @param manager a reference to motion
   */
  JointCommandSender(const ModuleManagerInterface& manager);
  /**
   * @brief cycle uses some joint commands from a motion module and sends them to the DCM
   */
  void cycle();

private:
  /// a reference to the motion request
  const Dependency<MotionRequest> motionRequest_;
  /// a reference to the motion activation
  const Dependency<MotionActivation> motionActivation_;
  /// a reference to the fall manager output
  const Dependency<FallManagerOutput> fallManagerOutput_;
  /// a reference to the head motion output
  const Dependency<HeadMotionOutput> headMotionOutput_;
  /// a reference to the jump output
  const Dependency<JumpOutput> jumpOutput_;
  /// a reference to the kick output
  const Dependency<KickOutput> kickOutput_;
  /// a reference to the point output
  const Dependency<PointOutput> pointOutput_;
  /// a reference to the poser output
  const Dependency<PoserOutput> poserOutput_;
  /// a reference to the stand up output
  const Dependency<StandUpOutput> standUpOutput_;
  /// a reference to the sit down output
  const Dependency<SitDownOutput> sitDownOutput_;
  /// a reference to the sit up output
  const Dependency<SitUpOutput> sitUpOutput_;
  /// a reference to the walking engine walk output
  const Dependency<WalkingEngineWalkOutput> walkingEngineWalkOutput_;
  /// a reference to the walking engine stand output
  const Dependency<WalkingEngineStandOutput> walkingEngineStandOutput_;
  /// a reference to the joint sensor data
  const Dependency<JointSensorData> jointSensorData_;
  /// a reference to the joint calibration data
  const Dependency<JointCalibrationData> jointCalibrationData_;
  /// a reference to the damage state
  const Dependency<BodyDamageData> bodyDamageData_;
  /// a reference to the motion state
  Production<MotionState> motionState_;
  /// the currently sent joint angles
  std::vector<float> angles_;
  /// the angles containing the calibration offsets
  std::vector<float> calibratedAngles_;
  /// the currently sent joint stiffnesses
  std::vector<float> stiffnesses_;
  /// the joint angles when interpolation started
  std::vector<float> startInterpolationAngles_;
};
