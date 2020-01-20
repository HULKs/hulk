#pragma once

#include "Behavior/BehaviorUnits/Head/ActiveVision.hpp"
#include "Behavior/Units.hpp"


ActionCommand setPlayStriker(const DataSet& d)
{
  if (d.setPlayStrikerAction.valid)
  {
    const Pose relPlayingPose = d.robotPosition.fieldToRobot(Pose(d.setPlayStrikerAction.kickPose));
    // select walk mode
    const float distanceThreshold = 1.5f;
    const float angleThreshold = 30 * TO_RAD;
    const WalkMode walkMode = SelectWalkMode::pathOrPathWithOrientation(
        relPlayingPose, distanceThreshold, angleThreshold);
    // code duplication > 9000
    switch (d.setPlayStrikerAction.type)
    {
      case SetPlayStrikerAction::Type::PASS:
        return walkToBallAndKick(d, d.setPlayStrikerAction.kickPose,
                                 d.setPlayStrikerAction.kickable, d.setPlayStrikerAction.target,
                                 true, Velocity(), d.setPlayStrikerAction.kickType)
            .combineHead(activeVision(d, VisionMode::BALL_TRACKER));
      case SetPlayStrikerAction::Type::DRIBBLE:
        return dribble(d, d.strikerAction.kickPose);
      case SetPlayStrikerAction::Type::IN_WALK_KICK:
        return walkToBallAndInWalkKick(d, d.setPlayStrikerAction.kickPose,
                                       d.setPlayStrikerAction.kickable,
                                       d.setPlayStrikerAction.inWalkKickType);
      case SetPlayStrikerAction::Type::WALK:
        return walkToPose(d, d.setPlayStrikerAction.kickPose, true, walkMode)
            .combineHead(activeVision(d, VisionMode::BALL_TRACKER));
      case SetPlayStrikerAction::Type::KICK:
      default:
        return walkToBallAndKick(d, d.setPlayStrikerAction.kickPose,
                                 d.setPlayStrikerAction.kickable, d.setPlayStrikerAction.target,
                                 true, Velocity(), d.setPlayStrikerAction.kickType)
            .combineHead(activeVision(d, VisionMode::BALL_TRACKER));
    }
  }
  else
  {
    Log(LogLevel::WARNING) << "invalid set play striker action";
    return ActionCommand::stand().combineHead(activeVision(d, VisionMode::BALL_TRACKER));
  }
}

ActionCommand striker(const DataSet& d)
{
  // It can happen that a striker does not see the ball by itself but due to uncertainty in ball and
  // robot position, the ball is behind the robot even though it thinks it should be in front of it.
  // ballState.found is also checked because it might be that the ball is not in the team ball
  // buffer when walking around the ball. This would lead to ballType becoming TEAM when another
  // robot sees the ball, but ballState.found will probably still be true.
  if (d.teamBallModel.ballType != TeamBallModel::BallType::SELF && !d.ballState.found &&
      (d.teamBallModel.position - d.robotPosition.pose.position).squaredNorm() < 0.5f * 0.5f)
  {
    return rotate(d).combineHead(activeVision(d, VisionMode::LOOK_FORWARD));
  }
  if (d.gameControllerState.setPlay != SetPlay::NONE)
  {
    return setPlayStriker(d);
  }
  if (d.strikerAction.valid)
  {
    // Check how many teamPlayers are already inside our penaltyArea
    uint8_t teamPlayersInsideOwnPenaltyArea{0};
    for (const TeamPlayer& teamPlayer : d.teamPlayers.players)
    {
      // We only want to count the other teamPlayers, not ourselves
      if (!(d.worldState.robotInPenaltyArea && d.worldState.robotInOwnHalf) &&
          teamPlayer.insideOwnPenaltyArea)
      {
        teamPlayersInsideOwnPenaltyArea++;
      }
    }
    // If there are 2 or more inside, we want to track the ball and walk to our own penaltySpot
    if (teamPlayersInsideOwnPenaltyArea >= 2 && d.worldState.ballInPenaltyArea &&
        d.worldState.ballInOwnHalf)
    {
      float orientation =
          std::atan2((d.robotPosition.pose.position.y() - d.teamBallModel.position.y()),
                     (d.robotPosition.pose.position.x() - d.teamBallModel.position.x()));
      Pose pose =
          Pose(-(d.fieldDimensions.fieldLength / 2) + d.fieldDimensions.fieldPenaltyMarkerDistance,
               orientation);

      return walkToPose(d, pose, true).combineHead(activeVision(d, VisionMode::BALL_TRACKER));
    }

    switch (d.strikerAction.type)
    {
      case StrikerAction::Type::PASS:
        return walkToBallAndKick(d, d.strikerAction.kickPose, d.strikerAction.kickable,
                                 d.strikerAction.target, true, Velocity(),
                                 d.strikerAction.kickType);
      case StrikerAction::Type::DRIBBLE:
        return dribble(d, d.strikerAction.kickPose);
      case StrikerAction::Type::IN_WALK_KICK:
        return walkToBallAndInWalkKick(d, d.strikerAction.kickPose, d.strikerAction.kickable,
                                       d.strikerAction.inWalkKickType);
      case StrikerAction::Type::WALK:
        return walkToPose(d, d.strikerAction.kickPose, true);
      case StrikerAction::Type::KICK:
      default:
        return walkToBallAndKick(d, d.strikerAction.kickPose, d.strikerAction.kickable,
                                 d.strikerAction.target, true, Velocity(), d.strikerAction.kickType)
            .combineHead(activeVision(d, VisionMode::BALL_TRACKER));
    }
  }
  else
  {
    Log(LogLevel::WARNING) << "Invalid striker action";
    return ActionCommand::stand().combineHead(activeVision(d, VisionMode::LOOK_AROUND));
  }
}
