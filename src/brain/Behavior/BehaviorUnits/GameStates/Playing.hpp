#pragma once
#include "Behavior/Units.hpp"

ActionCommand playSoccer(const DataSet& d)
{
  if (d.parameters.debugTargetEnable())
  {
    // Set a debug target pose which can be set by config.
    return ActionCommand::walk(d.parameters.debugTargetRelativePose())
        .combineHead(activeVision(d, VisionMode::BALL_TRACKER))
        .combineRightLED(ActionCommand::LED::white());
  }
  if (d.gameControllerState.gamePhase == GamePhase::PENALTYSHOOT)
  {
    return penaltyShootoutPlaying(d);
  }
  if (d.playingRoles.role == PlayingRole::KEEPER)
  {
    // the keeper should always be a keeper
    return keeper(d).combineRightLED(ActionCommand::LED::blue());
  }
  // Avoid illegal defender
  if (!d.worldState.ballIsFree)
  {
    // Stand and not rotate because rotating might lead to touching the center circle in certain
    // circumstances.
    return ActionCommand::stand().combineHead(activeVision(d, VisionMode::BALL_TRACKER));
  }
  // If we are a pass target and no striker, then we want to look at the teamball
  if (d.playingRoles.role != PlayingRole::STRIKER)
  {
    for (auto& teamPlayer : d.teamPlayers.players)
    {
      if (!teamPlayer.penalized &&
          teamPlayer.currentPassTarget == static_cast<int>(d.playerConfiguration.playerNumber) &&
          teamPlayer.currentlyPerformingRole == PlayingRole::STRIKER &&
          d.cycleInfo.getTimeDiff(teamPlayer.timeWhenReachBallStriker) < 5.29f)
      {
        const Vector2f relBallPosition = d.robotPosition.fieldToRobot(d.teamBallModel.position);
        const float relBallAngle = std::atan2(relBallPosition.y(), relBallPosition.x());
        return walkToPose(d, Pose(0, 0, relBallAngle), false)
            .combineHead(activeVision(d, VisionMode::BALL_TRACKER))
            .combineRightLED(ActionCommand::LED::off());
      }
    }
  }
  if (d.teamBallModel.ballType != TeamBallModel::BallType::NONE &&
      d.playingRoles.role != PlayingRole::NONE)
  {
    // We know where the Ball is
    switch (d.playingRoles.role)
    {
      case PlayingRole::STRIKER:
        return striker(d).combineRightLED(ActionCommand::LED::red());
      case PlayingRole::DEFENDER_LEFT:
      case PlayingRole::DEFENDER_RIGHT:
        return defender(d).combineRightLED(ActionCommand::LED::green());
      case PlayingRole::BISHOP:
        return bishop(d).combineRightLED(ActionCommand::LED::yellow());
      case PlayingRole::SUPPORT_STRIKER:
        return support(d).combineRightLED(ActionCommand::LED::pink());
      case PlayingRole::REPLACEMENT_KEEPER:
        return replacementKeeper(d).combineRightLED(ActionCommand::LED::lightblue());
      default:
        assert(false);
        return defender(d);
    }
  }
  else
  {
    return searchForBall(d).combineRightLED(ActionCommand::LED::blue());
  }
}

ActionCommand playing(const DataSet& d)
{
  switch (d.playerConfiguration.role)
  {
    case Role::DEMO:
      return demo(d);
    case Role::SHOOT_ON_HEAD_TOUCH:
      return shootOnHeadTouch(d);
    case Role::PLAYER:
      return playSoccer(d);
    default:
      return ActionCommand::stand();
  }
}
