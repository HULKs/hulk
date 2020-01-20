#pragma once
#include "Behavior/Units.hpp"

ActionCommand ready(const DataSet& d)
{
  const bool iAmKickOffStriker =
      (d.setPosition.position.x() > -d.fieldDimensions.fieldCenterCircleDiameter / 2);

  if (d.gameControllerState.secondaryTime < 6)
  {
    const float orientation = iAmKickOffStriker ? std::atan2(-d.robotPosition.pose.position.y(),
                                                             -d.robotPosition.pose.position.x())
                                                : 0;
    return rotate(d, orientation, true).combineHead(activeVision(d, VisionMode::LOCALIZATION));
  }
  // The robot that is going to perform the kickoff should face the center of the center circle. All
  // other robots should have orientation zero.
  const float orientation =
      iAmKickOffStriker ? std::atan2(-d.setPosition.position.y(), -d.setPosition.position.x()) : 0;
  const ActionCommand::LED ledCommand =
      d.setPosition.isKickoffPosition ? ActionCommand::LED::red() : ActionCommand::LED::blue();
  return walkToPose(d, Pose(d.setPosition.position, orientation), true, WalkMode::PATH, Velocity(),
                    3.f)
      .combineHead(activeVision(d, VisionMode::LOCALIZATION))
      .combineRightLED(ledCommand);
}
