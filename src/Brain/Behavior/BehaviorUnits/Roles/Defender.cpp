#include "Brain/Behavior/Units.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/SelectWalkMode.hpp"

ActionCommand defender(const DataSet& d)
{
  if (!d.defenderAction.valid)
  {
    Log<M_BRAIN>(LogLevel::WARNING) << "invalid defender action";
    return ActionCommand::stand().combineHead(activeVision(d, VisionMode::LOOK_AROUND));
  }
  switch (d.defenderAction.type)
  {
    case DefenderAction::Type::GENUFLECT: {
      return ActionCommand::jump(JumpOutput::Type::SQUAT);
    }
    case DefenderAction::Type::DEFEND: {
      if (d.defendingPosition.valid)
      {
        Pose relPlayingPose;
        // select walk and vision mode
        float distanceThreshold = 1.5f;
        float angleThreshold = 30 * TO_RAD;
        auto visionMode = VisionMode::LOOK_AROUND_BALL;

        if (d.teamBallModel.ballType == TeamBallModel::BallType::NONE)
        {
          // if ball position is unknown, look straight ahead
          Vector2f absGoalPosition = Vector2f((d.fieldDimensions.fieldLength * 0.5f) -
                                                  d.fieldDimensions.fieldPenaltyAreaLength,
                                              0.f);
          Vector2f relGoalPosition = d.robotPosition.fieldToRobot(absGoalPosition);
          const float relGoalAngle = std::atan2(relGoalPosition.y(), relGoalPosition.x());
          relPlayingPose =
              Pose(d.robotPosition.fieldToRobot(d.defendingPosition.position), relGoalAngle);
          visionMode = VisionMode::SEARCH_FOR_BALL;
        }
        else
        {
          // if ball position is known
          const float relBallAngle =
              std::atan2(d.teamBallModel.relPosition.y(), d.teamBallModel.relPosition.x());
          relPlayingPose =
              Pose(d.robotPosition.fieldToRobot(d.defendingPosition.position), relBallAngle);
        }

        // when enemy has corner kick, head of defender should track the ball
        // also, distance and angle threshold for choosing WalkMode should be different
        if (d.gameControllerState.setPlay == SetPlay::CORNER_KICK &&
            d.gameControllerState.gameState == GameState::PLAYING &&
            !d.gameControllerState.kickingTeam)
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

      Log<M_BRAIN>(LogLevel::WARNING) << "invalid defending position";
      return ActionCommand::stand().combineHead(activeVision(d, VisionMode::LOOK_AROUND));
    }
    default: {
      Log<M_BRAIN>(LogLevel::WARNING) << "Invalid defender action";
      return ActionCommand::stand().combineHead(activeVision(d, VisionMode::LOOK_AROUND));
    }
  }
}
