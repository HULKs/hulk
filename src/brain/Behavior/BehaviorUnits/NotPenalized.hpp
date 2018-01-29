#pragma once
#include "Behavior/Units.hpp"

ActionCommand notPenalized(const DataSet& d)
{
  if (d.gameControllerState.state == GameState::INITIAL)
  {
    return initial(d);
  }
  else if (d.gameControllerState.state == GameState::FINISHED)
  {
    return finished(d);
  }
  else if (d.bodyPose.fallen && d.cycleInfo.getTimeDiff(d.bodyPose.timeWhenFallen) > 1.f)
  {
    return standUp(d);
  }
  else if (d.bodyPose.fallen)
  {
    return ActionCommand::hold();
  }
  else
  {
    const float ballAge = d.cycleInfo.getTimeDiff(d.ballState.timeWhenLastSeen);
    ActionCommand::LED ballLED = ActionCommand::LED::off();
    if (ballAge < 0.3) {
      ballLED = ActionCommand::LED::red();
    } else if (ballAge < 1.f) {
      ballLED = ActionCommand::LED::yellow();
    } else if (ballAge < 2.5f) {
      ballLED = ActionCommand::LED::lightblue();
    } else if (ballAge < 5.f) {
      ballLED = ActionCommand::LED::blue();
    }
    if (d.gameControllerState.state == GameState::READY)
    {
      return ready(d).combineLeftLED(ballLED);
    }
    else if (d.gameControllerState.state == GameState::SET)
    {
      return set(d).combineLeftLED(ballLED);
    }
    else if (d.gameControllerState.state == GameState::PLAYING)
    {
      return playing(d).combineLeftLED(ballLED);
    }
    return ActionCommand::stand();
  }
}
