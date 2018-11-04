#include "KickConfigurationProvider.hpp"

KickConfigurationProvider::KickConfigurationProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , inWalkFrontKick_(*this, "inWalkFrontKick", [this] { configurationChanged_ = true; })
  , inWalkTurnKick_(*this, "inWalkTurnKick", [this] { configurationChanged_ = true; })
  , kickConfigurationData_(*this)
  , configurationChanged_(true)
{
  updateOutput();
}


void KickConfigurationProvider::cycle()
{
  if (configurationChanged_)
  {
    updateOutput();
  }
}

void KickConfigurationProvider::updateOutput()
{
  // update the output
  kickConfigurationData_->inWalkKicks[static_cast<int>(InWalkKickType::NONE)] = InWalkKick();
  kickConfigurationData_->inWalkKicks[static_cast<int>(InWalkKickType::FORWARD)] =
      inWalkFrontKick_();
  kickConfigurationData_->inWalkKicks[static_cast<int>(InWalkKickType::TURN)] = inWalkTurnKick_();

  // convert things to rad
  for (auto& inWalkKick : kickConfigurationData_->inWalkKicks)
  {
    inWalkKick.preStep.orientation *= TO_RAD;
    inWalkKick.kickStep.orientation *= TO_RAD;
    inWalkKick.kickDirectionAngle *= TO_RAD;
  }
  // reset the update trigger
  configurationChanged_ = false;
}
