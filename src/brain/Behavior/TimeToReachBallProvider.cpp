#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"

#include "TimeToReachBallProvider.hpp"


TimeToReachBallProvider::TimeToReachBallProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , fallenPenalty_(*this, "fallenPenalty", [] {})
  , strikerBonus_(*this, "strikerBonus", [] {})
  , ballNotSeenPenalty_(*this, "ballNotSeenPenalty", [] {})
  , bodyPose_(*this)
  , gameControllerState_(*this)
  , robotPosition_(*this)
  , strikerAction_(*this)
  , teamBallModel_(*this)
  , walkingEngineWalkOutput_(*this)
  , timeToReachBall_(*this)
{
}

void TimeToReachBallProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  // TODO: integrate setPlay
  if ((gameControllerState_->gameState != GameState::PLAYING &&
       gameControllerState_->gameState != GameState::READY &&
       gameControllerState_->gameState != GameState::SET) ||
      gameControllerState_->penalty != Penalty::NONE ||
      gameControllerState_->gamePhase != GamePhase::NORMAL ||
      teamBallModel_->ballType == TeamBallModel::BallType::NONE || !strikerAction_->valid)
  {
    return;
  }
  // the translational components of the distance to walk
  const float translationVelocity = walkingEngineWalkOutput_->maxVelocityComponents.position.x();
  assert(translationVelocity > 0);
  const Vector2f relBallPosition = teamBallModel_->position - robotPosition_->pose.position;
  const float walkTimeDistance = relBallPosition.norm() / translationVelocity;
  // the pure rotational component of the distance to walk
  const float rotationVelocity = walkingEngineWalkOutput_->maxVelocityComponents.orientation;
  assert(rotationVelocity > 0);
  const float ballOrientation = std::atan2(relBallPosition.y(), relBallPosition.x());
  const float rotateTimeDistance =
      Angle::angleDiff(ballOrientation, robotPosition_->pose.orientation) / rotationVelocity;
  // additional penalty if the robot is fallen
  const float fallenPenalty = bodyPose_->fallen ? fallenPenalty_() : 0.f;
  // the time it takes to walk around the ball
  const float walkAroundBallVelocity = walkingEngineWalkOutput_->walkAroundBallVelocity;
  assert(walkAroundBallVelocity > 0);
  const Vector2f ballToTarget = strikerAction_->target - teamBallModel_->position;
  const float ballToTargetOrientation = std::atan2(ballToTarget.y(), ballToTarget.x());
  const float walkAroundBallDistance =
      Angle::angleDiff(ballToTargetOrientation, ballOrientation) / walkAroundBallVelocity;
  // a penalty if we don't see the ball ourselfs
  const float ballNotSeenPenalty =
      teamBallModel_->ballType != TeamBallModel::BallType::SELF ? ballNotSeenPenalty_() : 0.f;
  // assembling all times to the final result
  timeToReachBall_->timeToReachBall = walkTimeDistance + rotateTimeDistance + fallenPenalty +
                                      walkAroundBallDistance + ballNotSeenPenalty;
  timeToReachBall_->timeToReachBallStriker =
      std::max(0.f, walkTimeDistance + rotateTimeDistance + fallenPenalty + walkAroundBallDistance -
                        strikerBonus_());
  timeToReachBall_->valid = true;
}
