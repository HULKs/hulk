#pragma once

#include <array>

#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FallManagerOutput.hpp"
#include "Data/KeeperOutput.hpp"
#include "Data/KickOutput.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/MotionRequest.hpp"
#include "Data/PoserOutput.hpp"
#include "Data/StandUpOutput.hpp"
#include "Data/WalkingEngineWalkOutput.hpp"
#include "Framework/Module.hpp"


class Motion;

class MotionDispatcher : public Module<MotionDispatcher, Motion>
{
public:
  /// the name of this module
  ModuleName name = "MotionDispatcher";
  /**
   * @brief MotionDispatcher initializes members
   * @param manager a reference to motion
   */
  MotionDispatcher(const ModuleManagerInterface& manager);
  /**
   * @brief cycle transforms the commands from the buffer to a data type
   */
  void cycle();

private:
  /// the output of the body pose estimation
  const Reference<BodyPose> bodyPose_;
  /// the output of the cycle info
  const Reference<CycleInfo> cycleInfo_;
  /// the output of the fall manager
  const Reference<FallManagerOutput> fallManagerOutput_;
  /// the output of the keeper
  const Reference<KeeperOutput> keeperOutput_;
  /// the output of the kick
  const Reference<KickOutput> kickOutput_;
  /// the output of the poser
  const Reference<PoserOutput> poserOutput_;
  /// the output of the stand up
  const Reference<StandUpOutput> standUpOutput_;
  /// the output of the walking engine
  const Reference<WalkingEngineWalkOutput> walkingEngineWalkOutput_;
  /// the motion internal data type for commands
  const Dependency<MotionRequest> motionRequest_;
  /// the commands for specific modules
  Production<MotionActivation> motionActivation_;
  /// the last motion that was active
  MotionRequest::BodyMotion lastActiveMotion_;
  /// a local version of the motion activations of the body
  std::array<float, static_cast<unsigned int>(MotionRequest::BodyMotion::NUM)> activations_;
  /// a local version of the activation of the head motion
  float headMotionActivation_;
  bool fallManagerActive_;
  TimePoint timeWhenFallManagerFinished_;
};
