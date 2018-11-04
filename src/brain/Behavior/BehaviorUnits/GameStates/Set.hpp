#pragma once

#include "Behavior/Units.hpp"
#include "Tools/Math/Angle.hpp"

ActionCommand set(const DataSet& d)
{
  if (d.gameControllerState.gamePhase == GamePhase::NORMAL && d.robotPosition.valid)
  {
    const Vector2f relativeFieldCenter = d.robotPosition.fieldToRobot(Vector2f::Zero());
    const float ballRadius = 0.05f;
    return ActionCommand::stand().combineHead(ActionCommand::Head::lookAt(
        Vector3f(relativeFieldCenter.x(), relativeFieldCenter.y(), ballRadius)));
  }
  else
  {
    return ActionCommand::stand().combineHead(trackBall(d, false, 40.f * TO_RAD));
  }
}
