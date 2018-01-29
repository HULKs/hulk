#include "Tools/Chronometer.hpp"

#include "DefendingPositionProvider.hpp"


DefendingPositionProvider::DefendingPositionProvider(const ModuleManagerInterface& manager)
  : Module(manager, "DefendingPositionProvider")
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , playingRoles_(*this)
  , robotPosition_(*this)
  , teamBallModel_(*this)
  , teamPlayers_(*this)
  , worldState_(*this)
  , defendingPosition_(*this)
  , goalCenter_(-fieldDimensions_->fieldLength * 0.5f, 0.0f)
  , minRadius_(Vector2f(fieldDimensions_->fieldPenaltyAreaLength, fieldDimensions_->fieldPenaltyAreaWidth * 0.5f).norm() + 0.15f)
{
}

void DefendingPositionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  if (gameControllerState_->state != GameState::PLAYING || playingRoles_->role != PlayingRole::DEFENDER || !teamBallModel_->seen)
  {
    return;
  }

  const TeamPlayer *keeper = nullptr, *otherDefender = nullptr;
  findRelevantTeamPlayers(keeper, otherDefender);

  // #oscillation
  const bool iAmFar = robotPosition_->pose.position.x() > 0;
  const bool otherIsFar = (otherDefender == nullptr) || (otherDefender->pose.position.x() > 0);
  if (otherDefender == nullptr || (!iAmFar && otherIsFar))
  {
    if (!worldState_->ballInOwnHalf)
    {
      defendingPosition_->position.x() = -fieldDimensions_->fieldLength * 0.5f + 2.0f;
      // #oscillation
      defendingPosition_->position.y() = (robotPosition_->pose.position.y() > 0) ? 1.0f : -1.0f;
    }
    else
    {
      const float radius = std::max(minRadius_, (teamBallModel_->position - goalCenter_).norm() * 0.5f);
      defendingPosition_->position = getCirclePosition(radius, worldState_->ballInLeftHalf);
    }
  }
  else
  {
    const float myY = robotPosition_->pose.position.y();
    const float otherY = otherDefender->pose.position.y();
    const bool iAmLeft = myY > otherY;
    if (iAmFar)
    {
      defendingPosition_->position.x() = -2.0f;
      defendingPosition_->position.y() = iAmLeft ? 1.0f : -1.0f;
    }
    else
    {
      if (!worldState_->ballInOwnHalf)
      {
        defendingPosition_->position.x() = -fieldDimensions_->fieldLength * 0.5f + 2.0f;
        defendingPosition_->position.y() = iAmLeft ? 1.0f : -1.0f;
      }
      else
      {
        const float radius = std::max(minRadius_, (teamBallModel_->position - goalCenter_).norm() * 0.5f);
        defendingPosition_->position = getCirclePosition(radius, iAmLeft);
      }
    }
  }
  defendingPosition_->valid = true;
}

void DefendingPositionProvider::findRelevantTeamPlayers(const TeamPlayer*& keeper, const TeamPlayer*& otherDefender) const
{
  for (auto& player : teamPlayers_->players)
  {
    if (player.penalized)
    {
      continue;
    }
    if (player.currentlyPerfomingRole == PlayingRole::DEFENDER)
    {
      otherDefender = &player;
    }
    else if (player.currentlyPerfomingRole == PlayingRole::KEEPER)
    {
      keeper = &player;
    }
  }
}

Vector2f DefendingPositionProvider::getCirclePosition(const float radius, const bool left)
{
  // (goalPoint + lambda * (teamBall - goalPoint) - goalCenter).squaredNorm() == radius^2
  // v = goalPoint - goalCenter
  // w = teamBall - goalPoint
  // (v + lambda * w).squaredNorm() == radius^2
  // (v.x + lambda * w.x)^2 + (v.y + lambda * w.y)^2 == radius^2
  // v.x^2 + 2 * lambda * v.x * w.x + lambda^2 * w.x^2 + v.y^2 + 2 * lambda * v.y * w.y + lambda^2 * w.y^2 == radius^2
  // (w.x^2 + w.y^2) * lambda^2 + 2 * (v.x * w.x + v.y * w.y) * lambda + (v.x^2 + v.y^2 - radius^2) == 0
  // lambda_1|2 = -(v.x * w.x + v.y * w.y) / (w.x^2 + w.y^2) +- sqrt(p^2 / 4 - (v.x^2 + v.y^2 - radius^2) / (w.x^2 + w.y^2)
  const Vector2f goalPoint(-fieldDimensions_->fieldLength * 0.5f, left ? 0.5f : -0.5f);
  const Vector2f v = goalPoint - goalCenter_;
  const Vector2f w = teamBallModel_->position - goalPoint;
  const float sqrW = w.squaredNorm();
  if (sqrW < 0.01f * 0.01f)
  {
    return goalPoint + Vector2f(1.f, 0.f) * radius;
  }
  const float p_2 = (v.x() * w.x() + v.y() * w.y()) / sqrW;
  const float lambda = -p_2 + std::sqrt(p_2 * p_2 - (v.squaredNorm() - radius * radius) / sqrW);
  return goalPoint + w * lambda;
}
