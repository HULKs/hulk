#include "KickConfigurationProvider.hpp"

KickConfigurationProvider::KickConfigurationProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , forwardKick_(*this, "forwardKick", [this] { configurationChanged_ = true; })
  , sideKick_(*this, "sideKick", [this] { configurationChanged_ = true; })
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
  // update the output for kicks
  kickConfigurationData_->kicks[static_cast<int>(KickType::NONE)] = KickConfiguration();
  kickConfigurationData_->kicks[static_cast<int>(KickType::FORWARD)] = forwardKick_();
  kickConfigurationData_->kicks[static_cast<int>(KickType::SIDE)] = sideKick_();

  // convert things to rad for kicks
  for (auto& kick : kickConfigurationData_->kicks)
  {
    kick.yawLeft2right *= TO_RAD;
    kick.shoulderRoll *= TO_RAD;
    kick.shoulderPitchAdjustment *= TO_RAD;
    kick.ankleRoll *= TO_RAD;
    kick.anklePitch *= TO_RAD;
  }

  // update the output for in walk kicks
  kickConfigurationData_->inWalkKicks[static_cast<int>(InWalkKickType::NONE)] = InWalkKick();
  kickConfigurationData_->inWalkKicks[static_cast<int>(InWalkKickType::FORWARD)] =
      inWalkFrontKick_();
  kickConfigurationData_->inWalkKicks[static_cast<int>(InWalkKickType::TURN)] = inWalkTurnKick_();

  // convert things to rad for in walk kicks
  for (auto& inWalkKick : kickConfigurationData_->inWalkKicks)
  {
    inWalkKick.preStep.orientation *= TO_RAD;
    inWalkKick.kickStep.orientation *= TO_RAD;
    inWalkKick.kickDirectionAngle *= TO_RAD;
  }

  // reset the update trigger
  configurationChanged_ = false;
}
