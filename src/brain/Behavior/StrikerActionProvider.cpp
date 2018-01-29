#include <cmath>

#include "Tools/BallUtils.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/Math/Random.hpp"

#include "StrikerActionProvider.hpp"


StrikerActionProvider::StrikerActionProvider(const ModuleManagerInterface& manager)
  : Module(manager, "StrikerActionProvider")
  , checkIfKeeperWantsToPlayBall_(*this, "checkIfKeeperWantsToPlayBall", [] {})
  , shootIntoGoal_(*this, "shootIntoGoal", [] {})
  , distanceToBallDribble_(*this, "distanceToBallDribble", [] {})
  , angleToBallDribble_(*this, "angleToBallDribble", [this] { angleToBallDribble_() *= TO_RAD; })
  , distanceToBallKick_(*this, "distanceToBallKick", [] {})
  , angleToBallKick_(*this, "angleToBallKick", [this] { angleToBallKick_() *= TO_RAD; })
  , useOnlyThisFoot_(*this, "useOnlyThisFoot", [] {})
  , ballState_(*this)
  , teamBallModel_(*this)
  , fieldDimensions_(*this)
  , robotPosition_(*this)
  , teamPlayers_(*this)
  , gameControllerState_(*this)
  , lastAction_(StrikerAction::DRIBBLE)
  , lastSign_(useOnlyThisFoot_())
  , lastPassTarget_(0)
  , penaltyTargetOffset_(0.f)
  , strikerAction_(*this)
{
  angleToBallDribble_() *= TO_RAD;
  angleToBallKick_() *= TO_RAD;
}

void StrikerActionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");
  if ((gameControllerState_->state != GameState::PLAYING && gameControllerState_->state != GameState::READY && gameControllerState_->state != GameState::SET) ||
      gameControllerState_->penalty != Penalty::NONE ||
      (gameControllerState_->secondary != SecondaryState::NORMAL && gameControllerState_->secondary != SecondaryState::PENALTYSHOOT) ||
      teamBallModel_->ballType == TeamBallModel::BallType::NONE)
  {
    penaltyTargetOffset_ = 0.f;
    return;
  }
  else if (gameControllerState_->secondary == SecondaryState::PENALTYSHOOT && gameControllerState_->state == GameState::PLAYING)
  {
    calculatePenaltyStrikerAction();
    return;
  }
  penaltyTargetOffset_ = 0.f;
  if (gameControllerState_->secondary != SecondaryState::NORMAL)
  {
    return;
  }

  if (checkIfKeeperWantsToPlayBall_() && keeperWantsToPlayBall())
  {
    // if keeper wants the ball, stay one meter away to wait for playing the ball
    // TODO: Do more intelligent stuff
    // By default stay where you are
    Pose newWalkPose(robotPosition_->pose);

    const float suggestedDistanceToBall = 1.0f;
    const Vector2f distanceVectorToBall = teamBallModel_->position - robotPosition_->pose.position;
    // if to close to ball, increase distance
    if (distanceVectorToBall.norm() <= suggestedDistanceToBall)
    {
      const Vector2f walkPosition = teamBallModel_->position - distanceVectorToBall * suggestedDistanceToBall / distanceVectorToBall.norm();
      newWalkPose = Pose(walkPosition, std::atan2(distanceVectorToBall.y(), distanceVectorToBall.x()));
    }

    // TODO: Handle case if robot is far away from ball; maybe twice or more than suggestedDistanceToBall?


    strikerAction_->target = Vector2f();
    strikerAction_->type = StrikerAction::WAITING_FOR_KEEPER;
    strikerAction_->kickPose = newWalkPose;
    strikerAction_->kickable = BallUtils::NOT;
    lastAction_ = strikerAction_->type;
    strikerAction_->valid = true;
    lastPassTarget_ = 0;
    return;
  }
  // do not change order
  calculateStrikerAction();
  calculateKick();
}


void StrikerActionProvider::calculatePenaltyStrikerAction()
{
  if (penaltyTargetOffset_ == 0.f)
  {
    // TODO: estimate a better target when there is a robot detection
    penaltyTargetOffset_ = Random::uniformInt(0, 1) == 0 ? -1.f : 1.f;
  }

  const Vector2f absBallPos = robotPosition_->robotToField(ballState_->position);
  const Vector2f penaltySpot = Vector2f(fieldDimensions_->fieldLength * 0.5f - fieldDimensions_->fieldPenaltyMarkerDistance, 0.f);

  const bool ballKickable = ballState_->found && (absBallPos - penaltySpot).norm() < 0.5f;
  if (ballKickable)
  {
    const Vector2f target =
        robotPosition_->fieldToRobot(Vector2f(fieldDimensions_->fieldLength * 0.5f, penaltyTargetOffset_ * fieldDimensions_->goalInnerWidth * 0.45f));
    int useOnlyThisFoot = 1; // One may want to use useOnlyThisFoot_() here, but especially in penalty shootouts the left foot seems to be better.
    const bool forceSign = useOnlyThisFoot != 0;
    int& lastSign = forceSign ? useOnlyThisFoot : lastSign_;
    const float distanceToBall = 0.20f;
    const float angleToBall = 5 * TO_RAD;
    const Pose kickPose = BallUtils::kickPose(ballState_->position, target, distanceToBall, lastSign);
    const BallUtils::Kickable kickable = BallUtils::kickable(kickPose, *ballState_, distanceToBall, angleToBall);
    strikerAction_->kickPose = kickPose;
    strikerAction_->type = StrikerAction::Type::KICK_INTO_GOAL;
    strikerAction_->kickType = StrikerAction::KickType::CLASSIC;
    strikerAction_->target = target;
    strikerAction_->kickable = kickable;
    strikerAction_->valid = true;
  }
  else
  {
    strikerAction_->valid = false;
  }
}

bool StrikerActionProvider::keeperWantsToPlayBall() const
{
  // check if keeper wants to play ball
  for (auto& teamPlayer : teamPlayers_->players)
  {
    if (teamPlayer.currentlyPerfomingRole == PlayingRole::KEEPER && teamPlayer.keeperWantsToPlayBall)
    {
      return true;
    }
  }
  return false;
}

void StrikerActionProvider::calculateStrikerAction()
{
  // gather some infos
  const Vector2f goalPosition(fieldDimensions_->fieldLength / 2.f, 0.f);
  const Vector2f ballTarget(goalPosition.x() + 0.2f, goalPosition.y());
  const Vector2f ballPosition = teamBallModel_->position;

  const bool wasDribblingIntoGoal = lastAction_ == StrikerAction::DRIBBLE_INTO_GOAL;
  const float dribbleIntoGoalThreshold = wasDribblingIntoGoal ? 0.6f : 0.5f;
  if (ballPosition.y() < dribbleIntoGoalThreshold && ballPosition.y() > -dribbleIntoGoalThreshold &&
      ballPosition.x() > (goalPosition.x() - dribbleIntoGoalThreshold))
  {
    strikerAction_->target = ballTarget;
    strikerAction_->type = StrikerAction::DRIBBLE_INTO_GOAL;
    lastAction_ = strikerAction_->type;
    strikerAction_->valid = true;
    lastPassTarget_ = 0;
    return;
  }

  const float ballRating = ratePosition(teamBallModel_->position);

  // check if I want to score
  const bool wasScoring = lastAction_ == StrikerAction::DRIBBLE_INTO_GOAL || lastAction_ == StrikerAction::KICK_INTO_GOAL;
  const float scoreThresh = wasScoring ? 3.f : 2.f;
  if (ballRating < scoreThresh)
  {
    strikerAction_->target = ballTarget;
    strikerAction_->type = shootIntoGoal_() ? StrikerAction::KICK_INTO_GOAL : StrikerAction::DRIBBLE_INTO_GOAL;
    lastAction_ = strikerAction_->type;
    strikerAction_->valid = true;
    lastPassTarget_ = 0;
    return;
  }

  // find a pass target
  const PassTarget passTarget = findPassTarget(ballRating);
  if (passTarget.number != 0)
  {
    strikerAction_->type = StrikerAction::PASS;
    strikerAction_->target = passTarget.position;
    strikerAction_->passTarget = passTarget.number;
    lastAction_ = strikerAction_->type;
    lastPassTarget_ = passTarget.number;
    strikerAction_->valid = true;
    return;
  }

  // fall back to dribble to a better position
  const Vector2f fallbackTarget(fieldDimensions_->fieldLength / 2.f - fieldDimensions_->fieldPenaltyAreaLength, 0.f);
  strikerAction_->type = StrikerAction::DRIBBLE;
  strikerAction_->target = fallbackTarget;
  lastAction_ = strikerAction_->type;
  lastPassTarget_ = 0;
  strikerAction_->valid = true;
}

void StrikerActionProvider::calculateKick()
{
  const Vector2f relBallSource = robotPosition_->fieldToRobot(teamBallModel_->position);
  const Vector2f relBallTarget = robotPosition_->fieldToRobot(strikerAction_->target);
  int useOnlyThisFoot = useOnlyThisFoot_();
  const bool forceSign = useOnlyThisFoot != 0;
  int& lastSign = forceSign ? useOnlyThisFoot : lastSign_;
  if (strikerAction_->type == StrikerAction::DRIBBLE_INTO_GOAL || strikerAction_->type == StrikerAction::DRIBBLE)
  {
    strikerAction_->kickPose = BallUtils::kickPose(relBallSource, relBallTarget, distanceToBallDribble_(), lastSign, forceSign);
    strikerAction_->kickable = BallUtils::kickable(strikerAction_->kickPose, *ballState_, distanceToBallDribble_(), angleToBallDribble_());
    strikerAction_->kickType = StrikerAction::KickType::IN_WALK_GENTLE;
    return;
  }
  strikerAction_->kickPose = BallUtils::kickPose(relBallSource, relBallTarget, distanceToBallKick_(), lastSign, forceSign);
  strikerAction_->kickable = BallUtils::kickable(strikerAction_->kickPose, *ballState_, distanceToBallKick_(), angleToBallKick_());
  strikerAction_->kickType = StrikerAction::KickType::CLASSIC;
}


float StrikerActionProvider::ratePosition(const Vector2f& position) const
{
  const Vector2f goalPosition(fieldDimensions_->fieldLength / 2.f, 0.f);
  const Vector2f positionToGoal = goalPosition - position;

  float distanceToGoal = positionToGoal.norm();
  float goalAngle = std::abs(std::atan2(positionToGoal.y(), positionToGoal.x()));
  return distanceToGoal + goalAngle * 0.75f;
}

StrikerActionProvider::PassTarget StrikerActionProvider::findPassTarget(const float ballRating) const
{
  PassTarget passTarget;
  passTarget.number = 0;
  passTarget.rating = ballRating - lastTargetBonus_; // treat the ball rating like a lastTarget
  for (auto& player : teamPlayers_->players)
  {
    if (player.penalized || player.fallen)
    {
      continue;
    }
    const float playerDistance = (player.pose.position - teamBallModel_->position).norm();
    if (playerDistance < 1.5f || playerDistance > 3.f)
    {
      continue;
    }
    const float bonus = (lastAction_ == StrikerAction::PASS && player.playerNumber == lastPassTarget_) ? lastTargetBonus_ : 0.f;
    const float playerRating = ratePosition(player.pose.position) - bonus;
    if (playerRating >= passTarget.rating)
    {
      continue;
    }
    passTarget.number = player.playerNumber;
    passTarget.rating = playerRating;
    passTarget.position = player.pose.position;
  }
  return passTarget;
}
