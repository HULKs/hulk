#include "Brain/Behavior/Units.hpp"
#include "Tools/SelectWalkMode.hpp"

ActionCommand bishop(const DataSet& d)
{
  // only use the bishop position if it is valid
  if (d.bishopPosition.valid)
  {
    const Pose relPlayingPose =
        d.robotPosition.fieldToRobot(Pose(d.bishopPosition.position, d.bishopPosition.orientation));

    // select walk and vision mode
    float distanceThreshold = 1.5f;
    float angleThreshold = 30 * TO_RAD;
    auto visionMode = VisionMode::LOOK_AROUND_BALL;

    // when enemy has any kind of enemy free kick, head of bishop should track the ball
    // also, distance and angle threshold for choosing WalkMode should be different
    if (d.gameControllerState.setPlay != SetPlay::NONE &&
        d.gameControllerState.gameState == GameState::PLAYING && !d.gameControllerState.kickingTeam)
    {
      visionMode = VisionMode::BALL_TRACKER;
      distanceThreshold = d.parameters.freeKickPathWithOrientationDistanceThreshold();
      angleThreshold = d.parameters.freeKickPathWithOrientationAngleThreshold();
    }

    const ActionCommand::Body::WalkMode walkMode = SelectWalkMode::pathOrPathWithOrientation(
        relPlayingPose, distanceThreshold, angleThreshold);

    return walkToPose(d, relPlayingPose, false, walkMode, Velocity(), 5)
        .combineHead(activeVision(d, visionMode));
  }
  Log<M_BRAIN>(LogLevel::WARNING) << "Invalid bishop position";
  return ActionCommand::stand().combineHead(activeVision(d, VisionMode::LOOK_AROUND));
}
