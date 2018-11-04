#pragma once

#include "Framework/Module.hpp"

#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/KickConfigurationData.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/MotionPlannerOutput.hpp"
#include "Data/MotionRequest.hpp"
#include "Data/WalkGenerator.hpp"
#include "Data/WalkingEngineStandOutput.hpp"
#include "Data/WalkingEngineWalkOutput.hpp"
#include "Tools/Kinematics/KinematicMatrix.h"

#include "StateHandling/RootOption.hpp"
#include "StateHandling/WalkManState.hpp"

class Motion;

class WalkManager : public Module<WalkManager, Motion>
{
public:
  ModuleName name = "WalkManager";

  WalkManager(const ModuleManagerInterface& manager);

  void cycle();

private:
  /// the minimum time the robot has to keep standing before we allow leaving
  const Parameter<float> minTimeInStandBeforeLeaving_;

  // the dependencies of this module
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<KickConfigurationData> kickConfigurationData_;
  const Dependency<MotionActivation> motionActivation_;
  const Dependency<MotionPlannerOutput> motionPlannerOutput_;
  const Dependency<MotionRequest> motionRequest_;
  const Dependency<WalkGenerator> walkGenerator_;

  // the productions of this module - angles and stiffnesses for the whole body except from head
  // TODO: these don't need to be separate outputs anymore, but this must be kept like this as long
  // as compatibily with the old walking engine is still required they will always contain the same
  Production<WalkingEngineWalkOutput> walkingEngineWalkOutput_;
  Production<WalkingEngineStandOutput> walkingEngineStandOutput_;

  // some extra private members to keep track of the state
  WalkManState wmState_;
  // the state machine handling the state transitions
  RootOption walkRootOption_;

  /**
   * @brief generateStandOutputFromWalkOutput generates an equivalent WalkingEngineStandOutput from a given WalkingEngineWalkOutput
   * @param walkOutput the WalkingEngineWalkOutput to be copied from
   * @return the generated stand output
   */
  WalkingEngineStandOutput& generateStandOutputFromWalkOutput(const WalkingEngineWalkOutput& walkOutput) const;
};
