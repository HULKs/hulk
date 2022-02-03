#include "Brain/Behavior/Units.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Tools/Math/Eigen.hpp"
#include <cmath>

ActionCommand ready(const DataSet& d)
{
  const bool iAmKickOffStriker =
      (d.setPosition.position.x() > -d.fieldDimensions.fieldCenterCircleDiameter / 2);
  float orientation{0.f};

  // If we are doing a penalty kick, look at the penalty spot
  if (d.gameControllerState.setPlay == SetPlay::PENALTY_KICK)
  {
    const float side = d.gameControllerState.kickingTeam ? 1.f : -1.f;
    const Vector2f absPenaltySpotLocation{Vector2f{
        side * (d.fieldDimensions.fieldLength / 2.f - d.fieldDimensions.fieldPenaltyMarkerDistance),
        0.f}};
    const Vector2f relativePenaltySpotLocation{absPenaltySpotLocation -
                                               d.robotPosition.pose.position()};
    orientation = std::atan2(relativePenaltySpotLocation.y(), relativePenaltySpotLocation.x());
  }
  else
  {
    // The robot that is going to perform the kickoff should face the center of the center circle.
    // All other robots should have orientation zero.
    orientation = iAmKickOffStriker
                      ? std::atan2(-d.setPosition.position.y(), -d.setPosition.position.x())
                      : 0.f;
  }

  if (d.gameControllerState.secondaryTime < 6)
  {
    if (d.gameControllerState.setPlay != SetPlay::PENALTY_KICK)
    {
      orientation = iAmKickOffStriker
                        ? std::atan2(-d.robotPosition.pose.y(), -d.robotPosition.pose.x())
                        : 0.f;
    }
    return rotate(d, orientation, true).combineHead(activeVision(d, VisionMode::LOCALIZATION));
  }
  const ActionCommand::LED ledCommand =
      d.setPosition.isKickoffPosition ? ActionCommand::LED::red() : ActionCommand::LED::blue();
  return walkToPose(d, Pose(d.setPosition.position, orientation), true,
                    ActionCommand::Body::WalkMode::PATH, Velocity(), 3.f)
      .combineHead(activeVision(d, VisionMode::LOCALIZATION))
      .combineRightLED(ledCommand);
}
