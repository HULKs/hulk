#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FallManagerOutput.hpp"
#include "Data/JumpOutput.hpp"
#include "Data/KickOutput.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/SitDownOutput.hpp"
#include "Data/SitUpOutput.hpp"
#include "Data/StandUpOutput.hpp"
#include "Data/WalkGeneratorOutput.hpp"
#include "Framework/Module.hpp"
#include <array>


class Motion;

class MotionDispatcher : public Module<MotionDispatcher, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"MotionDispatcher"};
  /**
   * @brief MotionDispatcher initializes members
   * @param manager a reference to motion
   */
  explicit MotionDispatcher(const ModuleManagerInterface& manager);
  /**
   * @brief cycle transforms the commands from the buffer to a data type
   */
  void cycle() override;

private:
  const Dependency<ActionCommand> actionCommand_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<FallManagerOutput> fallManagerOutput_;
  const Dependency<JumpOutput> jumpOutput_;
  const Dependency<KickOutput> kickOutput_;
  const Dependency<SitDownOutput> sitDownOutput_;
  const Dependency<SitUpOutput> sitUpOutput_;
  const Dependency<StandUpOutput> standUpOutput_;
  const Dependency<WalkGeneratorOutput> walkGeneratorOutput_;

  Production<MotionActivation> motionActivation_;

  /// the last motion that was active
  ActionCommand::Body::MotionType lastActiveMotion_;
  /// a local version of the motion activations of the body
  ActionCommand::Body::MotionTypeArray<float> activations_{};
  /// a local version of the activation of the head motion
  float headMotionActivation_ = 0.f;
  /// whether the fall manager is active
  bool fallManagerActive_ = false;
  /// time when the fall manager is finished
  Clock::time_point timeWhenFallManagerFinished_;
};
