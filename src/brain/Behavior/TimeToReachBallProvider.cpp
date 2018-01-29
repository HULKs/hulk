#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"

#include "TimeToReachBallProvider.hpp"


TimeToReachBallProvider::TimeToReachBallProvider(const ModuleManagerInterface& manager)
  : Module(manager, "TimeToReachBallProvider")
  , translationVelocity_(*this, "translationVelocity", [] {})
  , rotationVelocity_(*this, "rotationVelocity", [this] { rotationVelocity_() *= TO_RAD; })
  , walkAroundBallVelocity_(*this, "walkAroundBallVelocity", [this] { walkAroundBallVelocity_() *= TO_RAD; })
  , fallenPenalty_(*this, "fallenPenalty", [] {})
  , strikerBonus_(*this, "strikerBonus", [] {})
  , ballNotSeenPenalty_(*this, "ballNotSeenPenalty", [] {})
  , bodyPose_(*this)
  , gameControllerState_(*this)
  , robotPosition_(*this)
  , strikerAction_(*this)
  , teamBallModel_(*this)
  , timeToReachBall_(*this)
{
  rotationVelocity_() *= TO_RAD;
  walkAroundBallVelocity_() *= TO_RAD;
}

void TimeToReachBallProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  if ((gameControllerState_->state != GameState::PLAYING && gameControllerState_->state != GameState::READY && gameControllerState_->state != GameState::SET) ||
      gameControllerState_->penalty != Penalty::NONE || gameControllerState_->secondary != SecondaryState::NORMAL ||
      teamBallModel_->ballType == TeamBallModel::BallType::NONE || !strikerAction_->valid)
  {
    return;
  }
  const Vector2f relBallPosition = teamBallModel_->position - robotPosition_->pose.position;
  const float walkTimeDistance = relBallPosition.norm() / translationVelocity_();

  const float ballOrientation = std::atan2(relBallPosition.y(), relBallPosition.x());
  const float rotateTimeDistance = Angle::angleDiff(ballOrientation, robotPosition_->pose.orientation) / rotationVelocity_();

  const float fallenPenalty = bodyPose_->fallen ? fallenPenalty_() : 0.f;

  const Vector2f ballToTarget = strikerAction_->target - teamBallModel_->position;
  const float ballToTargetOrientation = std::atan2(ballToTarget.y(), ballToTarget.x());
  const float walkAroundBallDistance = Angle::angleDiff(ballToTargetOrientation, ballOrientation) / walkAroundBallVelocity_();

  const float ballNotSeenPenalty = teamBallModel_->ballType != TeamBallModel::BallType::SELF ? ballNotSeenPenalty_() : 0.f;

  timeToReachBall_->timeToReachBall = walkTimeDistance + rotateTimeDistance + fallenPenalty + walkAroundBallDistance + ballNotSeenPenalty;
  timeToReachBall_->timeToReachBallStriker = std::max(0.f, walkTimeDistance + rotateTimeDistance + fallenPenalty + walkAroundBallDistance - strikerBonus_());
  timeToReachBall_->valid = true;
}
