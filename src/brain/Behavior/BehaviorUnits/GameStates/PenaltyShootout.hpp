#pragma once
#include "Behavior/Units.hpp"

ActionCommand penaltyShootoutStriker(const DataSet& d)
{
  if (d.strikerAction.valid)
  {
    return walkToBallAndKick(d, d.strikerAction.kickPose, d.strikerAction.kickable, d.strikerAction.target, false, Velocity(0.05f, 1.f, false),
                             d.strikerAction.kickType)
        .combineHead(trackBall(d))
        .combineLeftLED(ActionCommand::LED::red());
  }
  else
  {
    const Vector2f penaltySpot = Vector2f(d.fieldDimensions.fieldLength / 2 - d.fieldDimensions.fieldPenaltyMarkerDistance, 0);
    const Vector2f relPenaltySpot = d.robotPosition.fieldToRobot(penaltySpot);
    return ActionCommand::stand().combineHead(ActionCommand::Head::lookAt({relPenaltySpot.x(), relPenaltySpot.y(), d.fieldDimensions.ballDiameter / 2}));
  }
}

ActionCommand penaltyShootoutPlaying(const DataSet& d)
{
  if (d.gameControllerState.kickoff)
  {
    return penaltyShootoutStriker(d).combineRightLED(ActionCommand::LED::red());
  }
  else
  {
    return keeper(d).combineRightLED(ActionCommand::LED::blue());
  }
}
