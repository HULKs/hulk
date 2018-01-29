#include "Tools/Chronometer.hpp"

#include "WhistleIntegration.hpp"


WhistleIntegration::WhistleIntegration(const ModuleManagerInterface& manager)
  : Module(manager, "WhistleIntegration")
  , minNumberOfAgreeingRobots_(*this, "minNumberOfAgreeingRobots", [] {})
  , rawGameControllerState_(*this)
  , whistleData_(*this)
  , teamPlayers_(*this)
  , cycleInfo_(*this)
  , gameControllerState_(*this)
  , prevGameState_(GameState::INITIAL)
  , prevRawGameState_(GameState::INITIAL)
  , prevSecondaryState_(SecondaryState::NORMAL)
{
}

void WhistleIntegration::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  *gameControllerState_ = *rawGameControllerState_;
  if (rawGameControllerState_->state == GameState::SET && rawGameControllerState_->secondary == SecondaryState::NORMAL)
  {
    if (prevRawGameState_ != GameState::SET || prevSecondaryState_ != SecondaryState::NORMAL)
    {
      lastTimeOfSet_ = cycleInfo_->startTime;
    }
    const unsigned int active = 1 + teamPlayers_->activePlayers;
    unsigned int agreeing = (whistleData_->lastTimeWhistleHeard > lastTimeOfSet_);
    for (auto& player : teamPlayers_->players)
    {
      if (!player.penalized && player.lastTimeWhistleHeard > lastTimeOfSet_)
      {
        agreeing++;
      }
    }
    if (agreeing >= std::min(active, minNumberOfAgreeingRobots_()))
    {
      if (prevGameState_ != GameState::PLAYING)
      {
        stateChanged_ = cycleInfo_->startTime;
      }
      gameControllerState_->state = GameState::PLAYING;
      gameControllerState_->stateChanged = stateChanged_;
    }
  }
  prevRawGameState_ = rawGameControllerState_->state;
  prevGameState_ = gameControllerState_->state;
  prevSecondaryState_ = rawGameControllerState_->secondary;
}
