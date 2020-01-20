#pragma once

#include "Behavior/Units.hpp"


enum class VisionMode
{
  LOOK_AROUND,
  LOOK_AROUND_BALL,
  BALL_TRACKER,
  LOCALIZATION,
  SEARCH_FOR_BALL,
  LOOK_FORWARD,
};

ActionCommand::Head activeVision(const DataSet& d, VisionMode mode)
{
  switch (mode)
  {
    case VisionMode::LOOK_AROUND:
      return ActionCommand::Head::angles(d.headPositionData.lookAroundHeadPosition,
                                         d.parameters.lookAroundYawVelocity());

    case VisionMode::LOOK_AROUND_BALL:
      return ActionCommand::Head::angles(d.headPositionData.lookAroundBallHeadPosition,
                                         d.parameters.lookAroundBallYawVelocity());

    case VisionMode::BALL_TRACKER:
      // always look at the team ball seen by this robot
      // look at team ball when this robot knows where it is (this can also be a ball of this robot)
      if (d.teamBallModel.ballType == TeamBallModel::BallType::SELF ||
          (d.robotPosition.valid &&
           (d.teamBallModel.seen || d.teamBallModel.ballType == TeamBallModel::BallType::RULE)))
      {
        const Vector2f relBallPosition = d.robotPosition.fieldToRobot(d.teamBallModel.position);
        return ActionCommand::Head::lookAt(
            {relBallPosition.x(), relBallPosition.y(), d.fieldDimensions.ballDiameter / 2});
      }
      // fall back to look around if there is no team ball at all or this robot does not know where
      // it is and has no own ball
      return ActionCommand::Head::angles(d.headPositionData.lookAroundHeadPosition,
                                         d.parameters.lookAroundYawVelocity());
    case VisionMode::LOCALIZATION:
      if (d.pointOfInterests.valid)
      {
        return ActionCommand::Head::lookAt(Vector3f(d.pointOfInterests.bestRelativePOI.position.x(),
                                                    d.pointOfInterests.bestRelativePOI.position.y(),
                                                    0.f));
      }
      return ActionCommand::Head::angles(d.headPositionData.lookAroundHeadPosition,
                                         d.parameters.lookAroundYawVelocity());

    case VisionMode::SEARCH_FOR_BALL:
      if (d.ballState.found)
      {
        return ActionCommand::Head::lookAt({d.ballState.position.x(), d.ballState.position.y(),
                                            d.fieldDimensions.ballDiameter / 2});
      }
      return ActionCommand::Head::angles(d.headPositionData.lookAroundHeadPosition,
                                         d.parameters.lookAroundYawVelocity());

    case VisionMode::LOOK_FORWARD:
    default:
      return ActionCommand::Head::angles(0, d.parameters.lookAroundOuterPosition()[1]);
  }
}
