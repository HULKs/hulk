#include "Brain/Behavior/Units.hpp"

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
  // handle behaviour during penalty kick after foul
  if (d.gameControllerState.setPlay == SetPlay::PENALTY_KICK)
  {
    if (d.playingRoles.role == PlayingRole::STRIKER && d.gameControllerState.kickingTeam)
    {
      return penaltyShootoutStriker(d).combineRightLED(ActionCommand::LED::red());
    }
    if (d.playerConfiguration.playerNumber == 1 && !d.gameControllerState.kickingTeam)
    {
      return penaltyKeeper(d).combineRightLED(ActionCommand::LED::blue());
    }
    return ActionCommand::stand().combineHead(activeVision(d, VisionMode::BALL_TRACKER));
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
    for (const auto& teamPlayer : d.teamPlayers.players)
    {
      if (!teamPlayer.penalized &&
          teamPlayer.currentPassTarget == static_cast<int>(d.playerConfiguration.playerNumber) &&
          teamPlayer.currentlyPerformingRole == PlayingRole::STRIKER &&
          d.cycleInfo.getAbsoluteTimeDifference(teamPlayer.timeWhenReachBallStriker) < 5s)
      {
        const float relBallAngle =
            std::atan2(d.teamBallModel.relPosition.y(), d.teamBallModel.relPosition.x());
        return walkToPose(d, Pose(0, 0, relBallAngle), false)
            .combineHead(activeVision(d, VisionMode::BALL_TRACKER))
            .combineRightLED(ActionCommand::LED::off());
      }
    }
  }
  switch (d.playingRoles.role)
  {
    case PlayingRole::STRIKER:
      return striker(d).combineRightLED(ActionCommand::LED::red());
    case PlayingRole::DEFENDER:
      return defender(d).combineRightLED(ActionCommand::LED::green());
    case PlayingRole::BISHOP:
      return bishop(d).combineRightLED(ActionCommand::LED::yellow());
    case PlayingRole::SUPPORT_STRIKER:
      return supporter(d).combineRightLED(ActionCommand::LED::pink());
    case PlayingRole::REPLACEMENT_KEEPER:
      return replacementKeeper(d).combineRightLED(ActionCommand::LED::lightblue());
    case PlayingRole::LOSER:
      return loser(d).combineRightLED(ActionCommand::LED::raspberry());
    case PlayingRole::SEARCHER:
      return searcher(d).combineRightLED(ActionCommand::LED::violet());
    default:
      assert(false);
      return defender(d);
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
