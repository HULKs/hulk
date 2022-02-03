#include "Brain/Behavior/Units.hpp"

ActionCommand notPenalized(const DataSet& d)
{
  // Special conditional checks introduced to enable penalty behavior.
  const bool isPenaltyWait = d.gameControllerState.gamePhase != GamePhase::PENALTYSHOOT ||
                             d.penaltyKeeperAction.type == PenaltyKeeperAction::Type::WAIT;

  if (d.gameControllerState.gameState == GameState::INITIAL)
  {
    return initial(d)
        .combineLeftLED(ActionCommand::LED::rainbow())
        .combineRightLED(ActionCommand::LED::rainbow());
  }
  if (d.bodyPose.fallen && isPenaltyWait && !d.sitDownOutput.isSitting)
  {
    // we still want to stand up even if the game is finished to be able to sit down correctly.
    // After finishing sit down we don't want to standUp anymore (even if we detected that we are
    // fallen).
    return standUp(d);
  }
  if (d.gameControllerState.gameState == GameState::FINISHED)
  {
    return finished(d);
  }

  const auto ballAge = d.cycleInfo.getAbsoluteTimeDifference(d.ballState.timeWhenLastSeen);
  ActionCommand::LED ballLED = ActionCommand::LED::off();
  if (ballAge < 0.3s)
  {
    ballLED = ActionCommand::LED::red();
  }
  else if (ballAge < 1s)
  {
    ballLED = ActionCommand::LED::yellow();
  }
  else if (ballAge < 2.5s)
  {
    ballLED = ActionCommand::LED::lightblue();
  }
  else if (ballAge < 5s)
  {
    ballLED = ActionCommand::LED::blue();
  }
  if (d.gameControllerState.gameState == GameState::READY)
  {
    return ready(d).combineLeftLED(ballLED);
  }
  if (d.gameControllerState.gameState == GameState::SET)
  {
    return set(d).combineLeftLED(ballLED);
  }
  if (d.gameControllerState.gameState == GameState::PLAYING)
  {
    return playing(d).combineLeftLED(ballLED);
  }
  return ActionCommand::stand();
}
