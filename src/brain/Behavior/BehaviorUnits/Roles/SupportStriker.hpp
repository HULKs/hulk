#pragma once

#include "Behavior/Units.hpp"
#include "Tools/SelectWalkMode.hpp"

ActionCommand support(const DataSet& d)
{
  // only use supporting position if it is valid
  if (d.supportingPosition.valid)
  {
    const Pose relPlayingPose = d.robotPosition.fieldToRobot(
        Pose(d.supportingPosition.position, d.supportingPosition.orientation));

    // select walk mode
    const float distanceThreshold = 1.5f;
    const float angleThreshold = 30 * TO_RAD;
    const WalkMode walkMode = SelectWalkMode::pathOrPathWithOrientation(
        relPlayingPose, distanceThreshold, angleThreshold);

    return walkToPose(d, relPlayingPose, false, walkMode)
        .combineHead(activeVision(d, VisionMode::LOOK_AROUND_BALL));
  }
  else
  {
    Log(LogLevel::WARNING) << "Invalid support striker position";
    return ActionCommand::stand().combineHead(activeVision(d, VisionMode::LOOK_AROUND));
  }
}
