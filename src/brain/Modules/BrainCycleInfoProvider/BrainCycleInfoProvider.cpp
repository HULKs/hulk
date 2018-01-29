#include "BrainCycleInfoProvider.hpp"


BrainCycleInfoProvider::BrainCycleInfoProvider(const ModuleManagerInterface& manager)
  : Module(manager, "BrainCycleInfoProvider")
  , imageData_(*this)
  , cycleInfo_(*this)
{
}

void BrainCycleInfoProvider::cycle()
{
  cycleInfo_->cycleTime = 0.01666f;
  cycleInfo_->startTime = imageData_->timestamp;
}
