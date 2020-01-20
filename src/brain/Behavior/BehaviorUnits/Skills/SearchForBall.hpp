#pragma once
#include "Behavior/Units.hpp"

ActionCommand searchForBall(const DataSet& d)
{
  Pose targetPose = {0.f, 0.f};
  // Fallback: don't move.
  if (d.robotPosition.valid)
  {
    targetPose = d.robotPosition.pose;
  }
  // real target pose if valid
  if (d.ballSearchPosition.ownSearchPoseValid)
  {
    targetPose = d.ballSearchPosition.pose;
  }
  else
  {
    Log(LogLevel::WARNING) << (int)d.playerConfiguration.playerNumber
                           << ": Own search pose is not valid! Falling back to stand!";
  }
  return walkToPose(d, targetPose, true, WalkMode::PATH, Velocity(), 5)
      .combineHead(activeVision(d, VisionMode::SEARCH_FOR_BALL));
}
