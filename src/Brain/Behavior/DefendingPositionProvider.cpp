#include "Data/PlayingRoles.hpp"
#include "Data/TeamBallModel.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Hysteresis.hpp"
#include "Tools/Math/Range.hpp"

#include "Brain/Behavior/DefendingPositionProvider.hpp"
#include <algorithm>


DefendingPositionProvider::DefendingPositionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , playingRoles_(*this)
  , robotPosition_(*this)
  , teamBallModel_(*this)
  , worldState_(*this)
  , defendingPosition_(*this)
  , passiveDefenseLineX_(-fieldDimensions_->fieldLength / 2.f +
                         fieldDimensions_->fieldPenaltyMarkerDistance - 0.3f)
  , passiveDefenseLineY_(fieldDimensions_->fieldGoalBoxAreaWidth / 2.f + 0.4f)
{
}

void DefendingPositionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  calculateDefendingPosition();
  considerSetPlay();
}

void DefendingPositionProvider::calculateDefendingPosition()
{
  if (gameControllerState_->gameState != GameState::PLAYING ||
      playingRoles_->role != PlayingRole::DEFENDER)
  {
    return;
  }

  // if ball position is unknown, return to default defending position
  if (teamBallModel_->ballType == TeamBallModel::BallType::NONE)
  {
    defendingPosition_->position =
        Vector2f{-fieldDimensions_->fieldLength / 2.f + 1.2f,
                 worldState_->robotInLeftHalf ? fieldDimensions_->goalInnerWidth / 4.f
                                              : -fieldDimensions_->goalInnerWidth / 4.f};
    defendingPosition_->valid = true;
    return;
  }
  const Vector2f absOwnGoalPosition = Vector2f(-fieldDimensions_->fieldLength / 2, 0.f);
  const float minPositionX = -fieldDimensions_->fieldLength / 2 + 0.5f;
  // the ball position is artificially limited
  Vector2f clippedAbsBallPosition = {std::max(teamBallModel_->absPosition.x(), minPositionX),
                                     teamBallModel_->absPosition.y()};
  const Vector2f ownGoalToBall = clippedAbsBallPosition - absOwnGoalPosition;
  const Vector2f orthogonal = fieldDimensions_->goalInnerWidth / 4.f *
                              (worldState_->ballInLeftHalf ? 1.f : -1.f) *
                              Vector2f(-ownGoalToBall.y(), ownGoalToBall.x()).normalized();
  // the y position is computed from intersecting lines to make sure the defenders do not block the
  // sight of the keeper
  const Line<float> shiftedKeeperSightLine =
      Line<float>(clippedAbsBallPosition + orthogonal, absOwnGoalPosition + orthogonal);

  // robot should be on passive defense line
  defendingPosition_->position.x() = passiveDefenseLineX_;
  // position towards the ball and clipped if on passive defense line
  defendingPosition_->position.y() =
      std::clamp(shiftedKeeperSightLine.getY(defendingPosition_->position.x()),
                 -passiveDefenseLineY_, passiveDefenseLineY_);
  defendingPosition_->valid = true;
}

void DefendingPositionProvider::considerSetPlay()
{
  const bool enemyHasFreeKick =
      gameControllerState_->setPlay != SetPlay::NONE && !gameControllerState_->kickingTeam;

  // If there is no free kick there is nothing to do
  if (!enemyHasFreeKick)
  {
    return;
  }

  // If the team ball was not found, we are not able to go away from it
  if (!teamBallModel_->found)
  {
    return;
  }

  // vector to the ball (relative to the robot)
  const Vector2f ballToRobot = robotPosition_->pose.position() - teamBallModel_->absPosition;
  // vector from the target position to the ball
  const Vector2f ballToTarget = defendingPosition_->position - teamBallModel_->absPosition;
  // set the defending position's y coord to something that is not illegal.
  if (ballToRobot.norm() < 0.9f || ballToTarget.norm() < 0.9f)
  {
    const float side = ballToRobot.y() < 0.f ? -1.f : 1.f;
    const float newYCoord =
        teamBallModel_->absPosition.y() +
        (side * std::sqrt(abs(0.9f * 0.9f - ballToRobot.x() * ballToRobot.x())));

    defendingPosition_->position.y() = newYCoord;

    debug().update(mount_ + ".modifiedDefPos", defendingPosition_->position);
  }
}
