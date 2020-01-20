#include "Tools/Chronometer.hpp"
#include "Tools/Math/Hysteresis.hpp"
#include "Tools/Math/Range.hpp"

#include "SetPlayStrikerActionProvider.hpp"


SetPlayStrikerActionProvider::SetPlayStrikerActionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , ballState_(*this)
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , kickConfigurationData_(*this)
  , robotPosition_(*this)
  , teamBallModel_(*this)
  , teamPlayers_(*this)
  , worldState_(*this)
  , setPlayStrikerAction_(*this)
  , enableScoring_(*this, "enableScoring", [] {})
  , enablePassing_(*this, "enablePassing", [] {})
  , distanceToBallDribble_(*this, "distanceToBallDribble", [] {})
  , angleToBallDribble_(*this, "angleToBallDribble", [this] { angleToBallDribble_() *= TO_RAD; })
  , angleToBallKick_(*this, "angleToBallKick", [this] { angleToBallKick_() *= TO_RAD; })
  , cornerKickTargetOffset_(*this, "cornerKickTargetOffset", [] {})
  , shouldKick_(false)
  , lastSign_(1)
  , ballNearOpponentGoal_(false)
{
  angleToBallDribble_() *= TO_RAD;
  angleToBallKick_() *= TO_RAD;
}

void SetPlayStrikerActionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");
  if (gameControllerState_->gameState != GameState::PLAYING ||
      gameControllerState_->penalty != Penalty::NONE ||
      gameControllerState_->gamePhase != GamePhase::NORMAL ||
      gameControllerState_->setPlay == SetPlay::NONE)
  {
    return;
  }

  if (gameControllerState_->kickingTeam)
  {
    performFreeKick();
  }
  else
  {
    block();
  }
}

void SetPlayStrikerActionProvider::createStrikerAction(const Pose& walkTarget)
{
  setPlayStrikerAction_->type = StrikerAction::Type::WALK;
  setPlayStrikerAction_->kickPose = walkTarget;
  setPlayStrikerAction_->valid = true;
}

void SetPlayStrikerActionProvider::createStrikerAction(const Vector2f& absTarget,
                                                       const Vector2f& relBallPosition,
                                                       int& lastSign, const bool forceSign)
{
  setPlayStrikerAction_->type = StrikerAction::Type::DRIBBLE;
  setPlayStrikerAction_->target = absTarget;
  setPlayStrikerAction_->kickPose = BallUtils::kickPose(
      relBallPosition, robotPosition_->fieldToRobot(absTarget), distanceToBallDribble_().x(),
      lastSign, forceSign, distanceToBallDribble_().y());
  setPlayStrikerAction_->kickable = BallUtils::kickable(
      setPlayStrikerAction_->kickPose, *ballState_, distanceToBallDribble_().x(),
      angleToBallDribble_(), distanceToBallDribble_().y(), setPlayStrikerAction_->kickable);
  setPlayStrikerAction_->valid = true;
}

void SetPlayStrikerActionProvider::createStrikerAction(const KickType kickType,
                                                       const Vector2f& absTarget,
                                                       const Vector2f& relBallPosition,
                                                       int& lastSign, const bool forceSign)
{
  const auto kick = kickConfigurationData_->kicks[static_cast<int>(kickType)];

  setPlayStrikerAction_->type = StrikerAction::Type::KICK;
  setPlayStrikerAction_->kickType = kickType;
  setPlayStrikerAction_->target = absTarget;
  setPlayStrikerAction_->kickPose =
      BallUtils::kickPose(relBallPosition, robotPosition_->fieldToRobot(absTarget),
                          kick.distanceToBall.x(), lastSign, forceSign, kick.distanceToBall.y());
  setPlayStrikerAction_->kickable = BallUtils::kickable(
      setPlayStrikerAction_->kickPose, *ballState_, kick.distanceToBall.x(), angleToBallKick_(),
      kick.distanceToBall.y(), setPlayStrikerAction_->kickable);
  setPlayStrikerAction_->valid = true;
}

void SetPlayStrikerActionProvider::block()
{
  const Vector2f absBallPosition = teamBallModel_->position;
  const Vector2f absOwnGoalPosition = Vector2f(-fieldDimensions_->fieldLength / 2.0f, 0.0f);
  const Vector2f ownGoalToBall = absBallPosition - absOwnGoalPosition;
  Vector2f blockerPosition = absBallPosition - 0.85f * ownGoalToBall.normalized();
  const float xPositionLimit = fieldDimensions_->fieldLength / 2.0f - 0.3f;
  // clip blocker position so that it does not retreack back into our own goal
  blockerPosition.x() =
      Range<float>::clipToGivenRange(blockerPosition.x(), -xPositionLimit, xPositionLimit);
  const Vector2f blockerPositionToBall = absBallPosition - blockerPosition;
  const float orientation = std::atan2(blockerPositionToBall.y(), blockerPositionToBall.x());
  createStrikerAction(Pose(blockerPosition, orientation));
}

void SetPlayStrikerActionProvider::performFreeKick()
{
  const Vector2f relBallPos = robotPosition_->fieldToRobot(teamBallModel_->position);
  const Vector2f ballTarget = kickTarget();
  SetPlayStrikerAction::Type action = kickOrDribble();
  if (action == SetPlayStrikerAction::Type::KICK)
  {
    createStrikerAction(KickType::FORWARD, ballTarget, relBallPos, lastSign_, false);
  }
  else
  {
    createStrikerAction(ballTarget, relBallPos, lastSign_, false);
  }
}

Vector2f SetPlayStrikerActionProvider::kickTarget() const
{
  const Vector2f absOpponentGoal(fieldDimensions_->fieldLength / 2.0f, 0.0f);
  const Vector2f cornerKickTarget =
      Vector2f(fieldDimensions_->fieldLength / 2.0f - cornerKickTargetOffset_(), 0.0f);
  const bool ballInOpponentsCorner = worldState_->ballInCorner && !worldState_->ballInOwnHalf;
  if (gameControllerState_->setPlay == SetPlay::CORNER_KICK)
  {
    return cornerKickTarget;
  }
  else if (gameControllerState_->setPlay == SetPlay::GOAL_FREE_KICK)
  {
    return absOpponentGoal;
  }
  else if (gameControllerState_->setPlay == SetPlay::KICK_IN)
  {
    return ballInOpponentsCorner ? cornerKickTarget : absOpponentGoal;
  }
  else if (gameControllerState_->setPlay == SetPlay::PUSHING_FREE_KICK)
  {
    return ballInOpponentsCorner ? cornerKickTarget : absOpponentGoal;
  }
  return absOpponentGoal;
}

SetPlayStrikerAction::Type SetPlayStrikerActionProvider::kickOrDribble()
{
  // try to score if ball is close to opponent's goal (if enabled)
  const Vector2f absOpponentGoal = Vector2f(fieldDimensions_->fieldLength / 2.0f, 0.0f);
  const float distanceToOpponentGoal = (teamBallModel_->position - absOpponentGoal).norm();
  ballNearOpponentGoal_ =
      Hysteresis<float>::smallerThan(distanceToOpponentGoal, 3.0f, 0.25f, ballNearOpponentGoal_);
  const bool ballInOpponentsCorner = worldState_->ballInCorner && !worldState_->ballInOwnHalf;
  if (ballNearOpponentGoal_ && enableScoring_() && !ballInOpponentsCorner)
  {
    return SetPlayStrikerAction::Type::KICK;
  }

  // pass if there is a pass target (if enabled)
  for (const auto& player : teamPlayers_->players)
  {
    if (player.penalized || player.fallen)
    {
      continue;
    }
    shouldKick_ =
        Hysteresis<float>::greaterThan(player.pose.position.x(), 0.0f, 0.25f, shouldKick_);
    if (shouldKick_ && enablePassing_())
    {
      return SetPlayStrikerAction::Type::KICK;
    }
  }
  return SetPlayStrikerAction::Type::DRIBBLE;
}
