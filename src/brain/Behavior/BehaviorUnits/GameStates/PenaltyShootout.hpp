#pragma once
#include "Behavior/Units.hpp"
#include <cmath>

ActionCommand penaltyShootoutStriker(const DataSet& d)
{
  if (d.penaltyStrikerAction.valid)
  {
    return walkToBallAndKick(d, d.penaltyStrikerAction.kickPose, d.penaltyStrikerAction.kickable,
                             d.penaltyStrikerAction.target, false, Velocity(0.5f, 0.5f, true),
                             d.penaltyStrikerAction.kickType)
        .combineHead(trackBall(d))
        .combineLeftLED(ActionCommand::LED::red());
  }
  else
  {
    const Vector2f penaltySpot = Vector2f(
        d.fieldDimensions.fieldLength / 2 - d.fieldDimensions.fieldPenaltyMarkerDistance, 0);
    const Vector2f relPenaltySpot = d.robotPosition.fieldToRobot(penaltySpot);
    return ActionCommand::stand().combineHead(ActionCommand::Head::lookAt(
        {relPenaltySpot.x(), relPenaltySpot.y(), d.fieldDimensions.ballDiameter / 2}));
  }
}

ActionCommand penaltyKeeper(const DataSet& d)
{
  switch (d.penaltyKeeperAction.type)
  {
    case PenaltyKeeperAction::Type::GENUFLECT:
      return ActionCommand::keeper(MK_TAKE_FRONT).combineLeftLED(ActionCommand::LED::green());
    case PenaltyKeeperAction::Type::JUMP_LEFT:
      return ActionCommand::keeper(MK_JUMP_LEFT).combineLeftLED(ActionCommand::LED::red());
    case PenaltyKeeperAction::Type::JUMP_RIGHT:
      return ActionCommand::keeper(MK_JUMP_RIGHT).combineLeftLED(ActionCommand::LED::yellow());
    case PenaltyKeeperAction::Type::WAIT:
    default:
      return ActionCommand::stand().combineLeftLED(ActionCommand::LED::lightblue());
  }
  return ActionCommand::stand().combineLeftLED(ActionCommand::LED::lightblue());
}

ActionCommand penaltyShootoutPlaying(const DataSet& d)
{
  if (d.gameControllerState.kickingTeam)
  {
    return penaltyShootoutStriker(d).combineRightLED(ActionCommand::LED::red());
  }
  else
  {
    return penaltyKeeper(d).combineRightLED(ActionCommand::LED::blue());
  }
}
