#include "Brain/Behavior/Units.hpp"
#include "Tools/SelectWalkMode.hpp"

ActionCommand supporter(const DataSet& d)
{
  // only use supporting position if it is valid
  if (d.supportingPosition.valid)
  {
    const Pose relPlayingPose = d.robotPosition.fieldToRobot(
        Pose(d.supportingPosition.position, d.supportingPosition.orientation));

    // select walk and vision mode
    float distanceThreshold = 1.5f;
    float angleThreshold = 30 * TO_RAD;
    auto visionMode = VisionMode::LOOK_AROUND_BALL;

    // when enemy has any kind of enemy free kick, head of supporter should track the ball
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
  Log<M_BRAIN>(LogLevel::WARNING) << "Invalid supporter position";
  return ActionCommand::stand().combineHead(activeVision(d, VisionMode::LOOK_AROUND));
}
