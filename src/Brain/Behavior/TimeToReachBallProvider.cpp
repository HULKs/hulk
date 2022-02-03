#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"

#include "Brain/Behavior/TimeToReachBallProvider.hpp"


TimeToReachBallProvider::TimeToReachBallProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , fallenPenalty_(*this, "fallenPenalty", [this] { generateEstimateTimeToReachBallFunction(); })
  , strikerBonus_(*this, "strikerBonus", [] {})
  , ballNotSeenPenalty_(*this, "ballNotSeenPenalty",
                        [this] { generateEstimateTimeToReachBallFunction(); })
  , walkAroundBallVelocityFactor_(*this, "walkAroundBallVelocityFactor", [] {})
  , bodyPose_(*this)
  , setPlayStrikerAction_(*this)
  , gameControllerState_(*this)
  , robotPosition_(*this)
  , strikerAction_(*this)
  , teamBallModel_(*this)
  , walkGeneratorOutput_(*this)
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
      robotPosition_->pose, teamBallModel_->absPosition, strikerAction_->target, bodyPose_->fallen,
      teamBallModel_->ballType == TeamBallModel::BallType::SELF,
      walkGeneratorOutput_->maxVelocityComponents);

  timeToReachBall_->timeToReachBallStriker =
      std::max(Clock::duration{0s}, timeToReachBall_->timeToReachBall - strikerBonus_());
  timeToReachBall_->valid = true;
}

void TimeToReachBallProvider::generateEstimateTimeToReachBallFunction()
{

  timeToReachBall_->estimateTimeToReachBall =
      [this](const Pose& playerPose, const Vector2f& ballPosition, const Vector2f& target,
             bool fallen, bool ballSeen, Pose maxVelocityComponents) -> Clock::duration {
    // the translational components of the distance to walk
    const float translationVelocity = maxVelocityComponents.x();
    assert(translationVelocity > 0);
    const Vector2f relBallPosition = ballPosition - playerPose.position();
    const auto walkTimeDistance =
        std::chrono::duration<float>(relBallPosition.norm() / translationVelocity);
    // the pure rotational component of the distance to walk
    const float rotationVelocity = maxVelocityComponents.angle();
    assert(rotationVelocity > 0);
    const float ballOrientation = std::atan2(relBallPosition.y(), relBallPosition.x());
    const auto rotateTimeDuration = std::chrono::duration<float>(
        Angle::angleDiff(ballOrientation, playerPose.angle()) / rotationVelocity);
    // additional penalty if the robot is fallen
    const auto fallenPenalty = fallen ? fallenPenalty_() : 0s;
    // the time it takes to walk around the ball
    const float estimatedTimeToWalkAroundBall =
        maxVelocityComponents.y() * walkAroundBallVelocityFactor_();
    assert(estimatedTimeToWalkAroundBall > 0);
    const Vector2f ballToTarget = target - ballPosition;
    const float ballToTargetOrientation = std::atan2(ballToTarget.y(), ballToTarget.x());
    const auto walkAroundBallDuration = std::chrono::duration<float>(
        Angle::angleDiff(ballToTargetOrientation, ballOrientation) / estimatedTimeToWalkAroundBall);
    // a penalty if we don't see the ball ourselfs
    const auto ballNotSeenPenalty = ballSeen ? ballNotSeenPenalty_() : 0s;
    // assembling all times to the final result
    return std::chrono::duration_cast<Clock::duration>(walkTimeDistance + rotateTimeDuration +
                                                       fallenPenalty + walkAroundBallDuration +
                                                       ballNotSeenPenalty);
  };
}
