#include "Tools/Chronometer.hpp"

#include "GameControllerModifier.hpp"


GameControllerModifier::GameControllerModifier(const ModuleManagerInterface& manager)
  : Module(manager)
  , enableWhistleIntegration_(*this, "enableWhistleIntegration", [] {})
  , minNumOfDetectedWhistles_(*this, "minNumOfDetectedWhistles", [] {})
  , rawGameControllerState_(*this)
  , whistleData_(*this)
  , teamPlayers_(*this)
  , cycleInfo_(*this)
  , gameControllerState_(*this)
  , prevRawGameControllerState_()
  , prevGameControllerState_()
{
}

void GameControllerModifier::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time_overall");
  *gameControllerState_ = *rawGameControllerState_;
  if (enableWhistleIntegration_())
  {
    integrateWhistle();
  }

  gameControllerState_->valid = true;
  prevRawGameControllerState_ = *rawGameControllerState_;
  prevGameControllerState_ = *gameControllerState_;
}

void GameControllerModifier::integrateWhistle()
{
  Chronometer time(debug(), mount_ + ".cycle_time_whistleIntegration");
  // if there  is a normal kick-off the game is started with a whistle
  // thus we have to modify the RawGameControllerState in order to go to playing after whistle was
  // heard
  if (rawGameControllerState_->gameState == GameState::SET &&
      rawGameControllerState_->gamePhase == GamePhase::NORMAL)
  {
    if (prevRawGameControllerState_.gameState != GameState::SET ||
        rawGameControllerState_->penalty == Penalty::ILLEGAL_MOTION_IN_SET ||
        prevRawGameControllerState_.gamePhase != GamePhase::NORMAL)
    {
      lastTimeStartedWhistleDetection_ = cycleInfo_->startTime;
    }
    // if we got a ILLEGAL_MOTION_IN_SET penalty we detected playing by mistake. Thus we have to
    // reset the state.
    if (rawGameControllerState_->penalty == Penalty::ILLEGAL_MOTION_IN_SET)
    {
      // the referee called ILLEGAL_MOTION_IN_SET. Thus we apparently were not in playing.
      prevGameControllerState_.gameState = GameState::SET;
    }
    // active players are this robot plus all team mates that we know of from the spl message
    const unsigned int active = 1 + teamPlayers_->activePlayers;
    // figure out how many robots heard a whistle during this set phase
    auto agreeing = static_cast<unsigned int>(whistleData_->lastTimeWhistleHeard >
                                              lastTimeStartedWhistleDetection_);
    for (auto& player : teamPlayers_->players)
    {
      if (!player.penalized && player.lastTimeWhistleHeard > lastTimeStartedWhistleDetection_)
      {
        agreeing++;
      }
    }
    // if enough robots heard the whistle or we already decided to be in playing before, we modify
    // the gameState to be SET
    if (agreeing >= std::min(active, minNumOfDetectedWhistles_()) ||
        (prevGameControllerState_.gameState == GameState::PLAYING &&
         prevRawGameControllerState_.gameState == GameState::SET))
    {
      if (prevGameControllerState_.gameState != GameState::PLAYING)
      {
        // if we just went to playing (we just agreed on the whistle) we store the timestamp of this
        // event
        stateChanged_ = cycleInfo_->startTime;
      }
      gameControllerState_->gameState = GameState::PLAYING;
      gameControllerState_->gameStateChanged = stateChanged_;
    }
  }
}
