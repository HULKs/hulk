#pragma once

#include "Behavior/Units.hpp"

ActionCommand replacementKeeper(const DataSet& d)
{
  // only use replacement keeper action if it is valid
  if (d.replacementKeeperAction.action.valid)
  {
    switch (d.replacementKeeperAction.action.type)
    {
      case KeeperAction::Type::BLOCK_GOAL:
      {
        const Pose relPlayingPose = d.robotPosition.fieldToRobot(d.replacementKeeperAction.action.pose);

        // select walk mode
        const float distanceThreshold = 1.5f;
        const float angleThreshold = 30 * TO_RAD;
        const WalkMode walkMode = SelectWalkMode::pathOrPathWithOrientation(
            relPlayingPose, distanceThreshold, angleThreshold);

        return walkToPose(d, relPlayingPose, false, walkMode)
            .combineHead(activeVision(d, VisionMode::LookAroundBall));
      }
      case KeeperAction::Type::GENUFLECT:
      {
        return ActionCommand::keeper(MK_TAKE_FRONT);
      }
      default:
      {
        return ActionCommand::stand().combineHead(lookAround(d));
      }
    }
  }
  else
  {
    Log(LogLevel::WARNING) << "invalid replacement keeper action";
    return ActionCommand::stand().combineHead(trackBall(d));
  }
}
