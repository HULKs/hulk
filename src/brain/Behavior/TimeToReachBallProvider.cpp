#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"

#include "TimeToReachBallProvider.hpp"


TimeToReachBallProvider::TimeToReachBallProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , fallenPenalty_(*this, "fallenPenalty", [this] { generateEstimateTimeToReachBallFunction(); })
  , strikerBonus_(*this, "strikerBonus", [] {})
  , ballNotSeenPenalty_(*this, "ballNotSeenPenalty",
                        [this] { generateEstimateTimeToReachBallFunction(); })
  , bodyPose_(*this)
  , setPlayStrikerAction_(*this)
  , gameControllerState_(*this)
  , robotPosition_(*this)
  , strikerAction_(*this)
  , teamBallModel_(*this)
  , walkingEngineWalkOutput_(*this)
  , timeToReachBall_(*this)
{
  generateEstimateTimeToReachBallFunction();
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
      teamBallModel_->ballType == TeamBallModel::BallType::NONE ||
      (!strikerAction_->valid && !setPlayStrikerAction_->valid))
  {
    return;
  }
  // assembling all times to the final result
  timeToReachBall_->timeToReachBall = timeToReachBall_->estimateTimeToReachBall(
      robotPosition_->pose, teamBallModel_->position, strikerAction_->target, bodyPose_->fallen,
      teamBallModel_->ballType == TeamBallModel::BallType::SELF,
      walkingEngineWalkOutput_->maxVelocityComponents,
      walkingEngineWalkOutput_->walkAroundBallVelocity);

  timeToReachBall_->timeToReachBallStriker =
      std::max(0.f, timeToReachBall_->timeToReachBall - strikerBonus_());
  timeToReachBall_->valid = true;
}

void TimeToReachBallProvider::generateEstimateTimeToReachBallFunction()
{

  timeToReachBall_->estimateTimeToReachBall =
      [this](Pose playerPose, Vector2f ballPosition, Vector2f target, bool fallen, bool ballSeen,
             Pose maxVelocityComponents, float walkAroundBallVelocity) -> float {
    // the translational components of the distance to walk
    const float translationVelocity = maxVelocityComponents.position.x();
    assert(translationVelocity > 0);
    const Vector2f relBallPosition = ballPosition - playerPose.position;
    const float walkTimeDistance = relBallPosition.norm() / translationVelocity;
    // the pure rotational component of the distance to walk
    const float rotationVelocity = maxVelocityComponents.orientation;
    assert(rotationVelocity > 0);
    const float ballOrientation = std::atan2(relBallPosition.y(), relBallPosition.x());
    const float rotateTimeDistance =
        Angle::angleDiff(ballOrientation, playerPose.orientation) / rotationVelocity;
    // additional penalty if the robot is fallen
    const float fallenPenalty = fallen ? fallenPenalty_() : 0.f;
    // the time it takes to walk around the ball
    assert(walkAroundBallVelocity > 0);
    const Vector2f ballToTarget = target - ballPosition;
    const float ballToTargetOrientation = std::atan2(ballToTarget.y(), ballToTarget.x());
    const float walkAroundBallDistance =
        Angle::angleDiff(ballToTargetOrientation, ballOrientation) / walkAroundBallVelocity;
    // a penalty if we don't see the ball ourselfs
    const float ballNotSeenPenalty = ballSeen ? ballNotSeenPenalty_() : 0.f;
    // assembling all times to the final result
    return walkTimeDistance + rotateTimeDistance + fallenPenalty + walkAroundBallDistance +
           ballNotSeenPenalty;
  };
}
