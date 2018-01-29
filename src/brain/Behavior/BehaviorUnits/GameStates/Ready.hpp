#pragma once
#include "Behavior/Units.hpp"

ActionCommand ready(const DataSet& d)
{
  if (d.gameControllerState.secondaryTime < 10)
  {
    return rotate(d, 0, true).combineHead(lookAround(d, 40.f * TO_RAD));
  }
  // This hack is especially for the keeper which would otherwise possibly collide with goal posts.
  if (d.setPosition.position.x() < -d.fieldDimensions.fieldLength * 0.5f + 0.5f &&
      std::abs(d.setPosition.position.y()) < d.fieldDimensions.goalInnerWidth * 0.5f &&
      std::abs(d.robotPosition.pose.position.y()) > 0.5f * d.fieldDimensions.goalInnerWidth)
  {
    return walkToPose(d, Pose(d.setPosition.position.x() + 0.25f, d.setPosition.position.y(), static_cast<float>(-M_PI)), true)
        .combineHead(lookAround(d, 40.f * TO_RAD))
        .combineRightLED(ActionCommand::LED::yellow());
  }
  const ActionCommand::LED ledCommand = d.setPosition.isKickoffPosition ? ActionCommand::LED::red() : ActionCommand::LED::blue();
  return walkToPose(d, Pose(d.setPosition.position, 0), true, WalkMode::PATH, Velocity(), 3.f)
      .combineHead(lookAround(d, 40.f * TO_RAD))
      .combineRightLED(ledCommand);
}
