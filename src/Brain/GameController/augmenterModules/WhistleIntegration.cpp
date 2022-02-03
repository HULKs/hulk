#include "Brain/GameController/augmenterModules/WhistleIntegration.hpp"
#include "Framework/Log/Log.hpp"


WhistleIntegration::WhistleIntegration(ModuleBase& module)
  : maxWhistleTimeDiff_(module, "maxWhistleTimeDiff", [] {})
  , maxWaitForReadyMessage_(module, "maxWaitForReadyMessage", [] {})
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

// NOLINTNEXTLINE(readability-function-cognitive-complexity)
void WhistleIntegration::integrateWhistle(const RawGameControllerState& rawGcState,
                                          GameControllerState& gcState)
{
  /* The whistle must be detected and integrated into the GameState in the following cases:
   *  - RawState = SET      : Signals start of PLAYING GameState
   *  - RawState = PLAYING  : Signals start of READY phase after a goal
   * Additionally, it can also signal the end of the game which we don't handle here.
   */
  if (!(rawGcState.gameState == GameState::SET || rawGcState.gameState == GameState::PLAYING) ||
      rawGcState.gamePhase != GamePhase::NORMAL)
  {
    return;
  }

  // If we changed to SET or PLAYING this cycle, start accepting whistles
  if ((prevRawGcState_.gameState == GameState::READY && rawGcState.gameState == GameState::SET) ||
      (prevRawGcState_.gameState == GameState::SET && rawGcState.gameState == GameState::PLAYING) ||
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
                cycleInfo_->getAbsoluteTimeDifference(player.lastTimeWhistleHeard) <
                    maxWhistleTimeDiff_());
      }));
  // add our own whistle data
  agreeing += static_cast<decltype(agreeing)>(
      (whistleData_->lastTimeWhistleHeard > lastTimeStartedWhistleDetection_ &&
       cycleInfo_->getAbsoluteTimeDifference(whistleData_->lastTimeWhistleHeard) <
           maxWhistleTimeDiff_())
          ? 1
          : 0);
  // if enough robots heard the whistle or we already decided to be in playing before, we modify
  // the gameState to be PLAYING
  if ((agreeing >= std::min(active, minNumOfDetectedWhistles_()) ||
       prevGcState_.gameState == GameState::PLAYING) &&
      prevRawGcState_.gameState == GameState::SET)
  {
    if (prevGcState_.gameState != GameState::PLAYING)
    {
      // if we just went to PLAYING (we just agreed on the whistle) we store the timestamp of this
      // event
      stateChanged_ = cycleInfo_->startTime;
    }
    gcState.gameState = GameState::PLAYING;
    gcState.gameStateChanged = stateChanged_;
  }
  // If we were in PLAYING and heard a whistle, change to READY
  else if ((agreeing >= std::min(active, minNumOfDetectedWhistles_()) ||
            prevGcState_.gameState == GameState::READY) &&
           prevRawGcState_.gameState == GameState::PLAYING)
  {
    if (prevGcState_.gameState != GameState::READY)
    {
      // if we just went to READY (we just agreed on the whistle) we store the timestamp of this
      // event
      stateChanged_ = cycleInfo_->startTime;
    }
    if (cycleInfo_->getAbsoluteTimeDifference(stateChanged_) < maxWaitForReadyMessage_())
    {
      if (prevGcState_.gameState != GameState::READY)
      {
        Log<M_BRAIN>(LogLevel::INFO) << "Changing to READY, heard whistle in PLAYING";
      }
      gcState.gameState = GameState::READY;
      gcState.gameStateChanged = stateChanged_;
      // secondaryTime must be set in READY
      gcState.secondaryTime = 45.f - std::chrono::duration_cast<std::chrono::seconds>(
                                         cycleInfo_->getAbsoluteTimeDifference(stateChanged_))
                                         .count();
      // Assume we are not kicking! Safe firstly!
      gcState.kickingTeam = false;
      gcState.kickingTeamNumber =
          1; // TODO This is a hack: we do not know the opponent's team number
    }
    else
    {
      Log<M_BRAIN>(LogLevel::WARNING)
          << "Changing back to PLAYING, no GC confirmation received after "
          << std::to_string(maxWaitForReadyMessage_().count()) << " sec.";
      gcState.gameState = GameState::PLAYING;
      gcState.gameStateChanged = cycleInfo_->startTime;
    }
  }
}
