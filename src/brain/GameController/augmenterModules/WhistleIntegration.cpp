
#include "WhistleIntegration.hpp"


WhistleIntegration::WhistleIntegration(ModuleBase& module)
  : maxWhistleTimeDiff_(module, "maxWhistleTimeDiff", [] {})
  , minNumOfDetectedWhistles_(module, "minNumOfDetectedWhistles", [] {})
  , cycleInfo_(module)
  , whistleData_(module)
  , teamPlayers_(module)
  , prevRawGcState_()
  , prevGcState_()
{
}

void WhistleIntegration::cycle(const RawGameControllerState& rawGcState,
                               GameControllerState& gcState)
{
  integrateWhistle(rawGcState, gcState);

  prevRawGcState_ = rawGcState;
  prevGcState_ = gcState;
}


void WhistleIntegration::integrateWhistle(const RawGameControllerState& rawGcState,
                                          GameControllerState& gcState)
{
  // if there  is a normal kick-off the game is started with a whistle
  // thus we have to modify the RawGameControllerState in order to go to playing after whistle was
  // heard. This behavior is inactive in all other game phases.
  if (rawGcState.gameState != GameState::SET || rawGcState.gamePhase != GamePhase::NORMAL)
  {
    return;
  }

  if (prevRawGcState_.gameState != GameState::SET ||
      rawGcState.penalty == Penalty::ILLEGAL_MOTION_IN_SET ||
      prevRawGcState_.gamePhase != GamePhase::NORMAL)
  {
    lastTimeStartedWhistleDetection_ = cycleInfo_->startTime;
  }
  // if we got a ILLEGAL_MOTION_IN_SET penalty we detected playing by mistake. Thus we have to
  // reset the state.
  if (rawGcState.penalty == Penalty::ILLEGAL_MOTION_IN_SET)
  {
    // the referee called ILLEGAL_MOTION_IN_SET. Thus we apparently were not in playing.
    prevGcState_.gameState = GameState::SET;
  }
  // active players are this robot plus all team mates that we know of from the spl message
  const unsigned int active = 1 + teamPlayers_->activePlayers;
  // figure out how many robots heard a whistle in the last seconds during this set phase
  auto agreeing = static_cast<unsigned int>(std::count_if(
      teamPlayers_->players.begin(), teamPlayers_->players.end(), [this](const TeamPlayer& player) {
        return (!player.penalized &&
                player.lastTimeWhistleHeard > lastTimeStartedWhistleDetection_ &&
                getTimeDiff(player.lastTimeWhistleHeard, cycleInfo_->startTime, TDT::SECS) <
                    maxWhistleTimeDiff_());
      }));
  // add our own whistle data
  agreeing += static_cast<decltype(agreeing)>(
      (whistleData_->lastTimeWhistleHeard > lastTimeStartedWhistleDetection_ &&
       getTimeDiff(whistleData_->lastTimeWhistleHeard, cycleInfo_->startTime, TDT::SECS) <
           maxWhistleTimeDiff_())
          ? 1
          : 0);
  // if enough robots heard the whistle or we already decided to be in playing before, we modify
  // the gameState to be PLAYING
  if (agreeing >= std::min(active, minNumOfDetectedWhistles_()) ||
      (prevGcState_.gameState == GameState::PLAYING && prevRawGcState_.gameState == GameState::SET))
  {
    if (prevGcState_.gameState != GameState::PLAYING)
    {
      // if we just went to playing (we just agreed on the whistle) we store the timestamp of this
      // event
      stateChanged_ = cycleInfo_->startTime;
    }
    gcState.gameState = GameState::PLAYING;
    gcState.gameStateChanged = stateChanged_;
  }
}
