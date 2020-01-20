#include "GameControllerAugmenter.hpp"


GameControllerAugmenter::GameControllerAugmenter(const ModuleManagerInterface& manager)
  : Module(manager)
  , enableWhistleIntegration_(*this, "enableWhistleIntegration", [] {})
  , enableRefereeMistakeIntegration_(*this, "enableRefereeMistakeIntegration", [] {})
  , rawGameControllerState_(*this)
  , cycleInfo_(*this)
  , gameControllerState_(*this)
  , whistleIntegration_(*this)
  , refereeMistakeIntegration_(*this)
{
}

void GameControllerAugmenter::cycle()
{
  *gameControllerState_ = *rawGameControllerState_;
  if (enableWhistleIntegration_())
  {
    whistleIntegration_.cycle(*rawGameControllerState_, *gameControllerState_);
  }

  if (enableRefereeMistakeIntegration_())
  {
    refereeMistakeIntegration_.cycle(*rawGameControllerState_, *gameControllerState_);
  }
}
