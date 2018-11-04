#pragma once

#include "Behavior/Units.hpp"
#include "Tools/SelectWalkMode.hpp"

ActionCommand keeper(const DataSet& d)
{
  // only use keeper action if it is valid
  if (d.keeperAction.action.valid)
  {
    switch (d.keeperAction.action.type)
    {
      case KeeperAction::Type::BLOCK_GOAL:
      {
        const Pose relPlayingPose = d.robotPosition.fieldToRobot(d.keeperAction.action.pose);

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
    Log(LogLevel::WARNING) << "invalid keeper action";
    return ActionCommand::stand().combineHead(trackBall(d));
  }
}
