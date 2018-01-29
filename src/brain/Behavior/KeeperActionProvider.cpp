#include <cmath>

#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"

#include "KeeperActionProvider.hpp"

KeeperActionProvider::KeeperActionProvider(const ModuleManagerInterface& manager)
  : Module(manager, "KeeperActionProvider")
  , mayGenuflect_(*this, "mayGenuflect", [] {})
  , teamBallModel_(*this)
  , ballState_(*this)
  , fieldDimensions_(*this)
  , robotPosition_(*this)
  , teamPlayers_(*this)
  , gameControllerState_(*this)
  , keeperAction_(*this)
  , lastAction_(KeeperAction::SEARCH_FOR_BALL)
  , keeperPosition_(Vector2f((-fieldDimensions_->fieldLength * 0.5f) + fieldDimensions_->fieldPenaltyAreaLength * 0.5f, 0.f))
{
}

void KeeperActionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  if (gameControllerState_->state != GameState::PLAYING)
  {
    return;
  }

  if (ballState_->found)
  {
    if (mayGenuflect_() && shouldGenuflect())
    {
      keeperAction_->type = KeeperAction::GENUFLECT;
      lastAction_ = keeperAction_->type;
      keeperAction_->valid = true;
      return;
    }
  }

  // Dangerous ball, go for it and kick it away
  // b.) check if, a pass can be done
  if (teamBallModel_->seen)
  {
    const bool wasKicking = lastAction_ == KeeperAction::KICK_BALL_ASAP_AWAY;
    const float dangerousBallDistanceStageRed = wasKicking ? 1.8f : 1.3f;
    const float defendingDistanceForKeeper = 2.0f;

    // I can see a ball and it is very close
    const float ballDistance = (teamBallModel_->position - keeperPosition_).norm();
    const bool closeToGoal = (keeperPosition_ - robotPosition_->pose.position).norm() < defendingDistanceForKeeper;

    if (ballDistance <= dangerousBallDistanceStageRed && closeToGoal)
    {
      // and it is in a very dangerous position - kick it away!
      Vector2f kickTarget;
      if (Angle::angleDiff(robotPosition_->pose.orientation, 0.0f) < (80 * TO_RAD))
      {
        kickTarget = robotPosition_->pose.position + Vector2f(std::cos(robotPosition_->pose.orientation), std::sin(robotPosition_->pose.orientation)) * 2;
      }
      keeperAction_->type = KeeperAction::KICK_BALL_ASAP_AWAY;
      keeperAction_->wantsToPlayBall = true;
      keeperAction_->target = kickTarget;
      lastAction_ = keeperAction_->type;
      keeperAction_->valid = true;
      return;
    }

    const bool wasGoingClosedToBall = lastAction_ == KeeperAction::GO_CLOSER_TO_CLOSE_BALL;
    const float dangerousBallDistanceStageYellow = wasGoingClosedToBall ? 3.5f : 3.f;

    // I can see the ball in dangerous position - move a little bit towards it!
    if (ballDistance <= dangerousBallDistanceStageYellow)
    {
      const float interceptXCoord = keeperPosition_.x();
      const float interceptYCoord =
          std::min(1.f, std::max(-1.f, teamBallModel_->position.y() / (teamBallModel_->position.x() + fieldDimensions_->fieldLength / 2) *
                                           (interceptXCoord + fieldDimensions_->fieldLength / 2)));
      const Vector2f interceptVec = Vector2f(interceptXCoord, interceptYCoord);
      float interceptAngle = std::atan2(teamBallModel_->position.y() - keeperPosition_.y(), teamBallModel_->position.x() - keeperPosition_.x());
      keeperAction_->type = KeeperAction::GO_CLOSER_TO_CLOSE_BALL;
      keeperAction_->walkPosition = Pose(interceptVec, interceptAngle);
      lastAction_ = keeperAction_->type;
      keeperAction_->valid = true;
      return;
    }
  }

  // Nothing dangerous, go back to playing position
  if (std::abs(robotPosition_->pose.position.y()) > fieldDimensions_->goalInnerWidth / 2)
  {
    keeperAction_->type = KeeperAction::GO_TO_DEFAULT_POS;
    keeperAction_->walkPosition = Pose(keeperPosition_.x(), keeperPosition_.y(), 0.0f);
    lastAction_ = keeperAction_->type;
    keeperAction_->valid = true;
    return;
  }

  // Hysteresis
  const float distanceToPoseSqr = 0.1f * 0.1f;
  const float angleDiffToPose = static_cast<float>(M_PI_4 / 4);
  if (((robotPosition_->pose.position - keeperPosition_).squaredNorm() <= distanceToPoseSqr) &&
      Angle::angleDiff(robotPosition_->pose.orientation, 0) <= angleDiffToPose)
  {
    keeperAction_->type = KeeperAction::SEARCH_FOR_BALL;
    lastAction_ = keeperAction_->type;
    keeperAction_->valid = true;
    return;
  }

  keeperAction_->type = KeeperAction::GO_TO_DEFAULT_POS;
  lastAction_ = keeperAction_->type;
  keeperAction_->walkPosition = Pose(keeperPosition_.x(), keeperPosition_.y(), 0.0f);
  keeperAction_->valid = true;
  return;
}

bool KeeperActionProvider::shouldGenuflect() const
{
  const float genuflectWidth = 0.4f;
  /// ball will come to stop in goal + tolerance
  const bool inGoal = robotPosition_->robotToField(ballState_->destination).x() < (-fieldDimensions_->fieldLength / 2.f + 0.3f) &&
                      std::abs(robotPosition_->robotToField(ballState_->destination).y()) < (fieldDimensions_->goalInnerWidth / 2.f + 0.3f);
  /// ball rolls in direction of goal
  const bool goalDirection = robotPosition_->pose.calculateGlobalOrientation(ballState_->velocity).x() < 0;
  /// robot does not look in direction of the goal
  const bool robotLooksForward = Angle::angleDiff(robotPosition_->pose.orientation, 0) <= (100 * TO_RAD);

  if (inGoal && goalDirection && std::abs(ballState_->destination.y()) < genuflectWidth && robotLooksForward)
  {
    return true;
  }
  return false;
}
