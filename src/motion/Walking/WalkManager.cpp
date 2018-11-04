#include "WalkManager.hpp"
#include "Modules/NaoProvider.h"
#include "Modules/Poses.h"

WalkManager::WalkManager(const ModuleManagerInterface& manager)
  : Module(manager)
  , minTimeInStandBeforeLeaving_(*this, "minTimeInStandBeforeLeaving", [] {})
  , cycleInfo_(*this)
  , bodyPose_(*this)
  , kickConfigurationData_(*this)
  , motionActivation_(*this)
  , motionPlannerOutput_(*this)
  , motionRequest_(*this)
  , walkGenerator_(*this)
  , walkingEngineWalkOutput_(*this)
  , walkingEngineStandOutput_(*this)
  , wmState_(*motionActivation_, *motionPlannerOutput_, *motionRequest_, *kickConfigurationData_,
             *bodyPose_, *walkGenerator_, *cycleInfo_, minTimeInStandBeforeLeaving_())
{
}

void WalkManager::cycle()
{
  *walkingEngineWalkOutput_ = walkRootOption_.run(wmState_);
  // infer the stand output from the walk output (this is needed for backards compatibilityj)
  static_cast<MotionOutput&>(*walkingEngineStandOutput_) =
      static_cast<const MotionOutput&>(*walkingEngineWalkOutput_);
}
