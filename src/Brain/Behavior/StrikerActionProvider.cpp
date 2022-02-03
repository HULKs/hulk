#include <cmath>

#include "Tools/BallUtils.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Hysteresis.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/Math/Random.hpp"
#include "Tools/Math/Range.hpp"

#include "Framework/Log/Log.hpp"

#include "Brain/Behavior/StrikerActionProvider.hpp"


StrikerActionProvider::StrikerActionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , angleToBallDribble_(*this, "angleToBallDribble", [this] { angleToBallDribble_() *= TO_RAD; })
  , angleToBallKick_(*this, "angleToBallKick", [this] { angleToBallKick_() *= TO_RAD; })
  , asapDeviationAngle_(*this, "asapDeviationAngle", [this] { asapDeviationAngle_() *= TO_RAD; })
  , distanceToBallDribble_(*this, "distanceToBallDribble", [] {})
  , dribbleMapInterpolationPoints_(*this, "dribbleMapInterpolationPoints", [] {})
  , kickAwayFromGoal_(*this, "kickAwayFromGoal", [] {})
  , kickIntoGoal_(*this, "kickIntoGoal", [] {})
  , kickOpeningAngle_(*this, "kickOpeningAngle",
                      [this] {
                        kickOpeningAngle_() *= TO_RAD;
                        assert(kickOpeningAngle_() < 180.f * TO_RAD);
                      })
  , kickRatingChunkWeights_(*this, "kickRatingChunkWeights", [] {})
  , kickRatingThreshold_(*this, "kickRatingThreshold", [] {})
  , ownGoalAreaRadius_(*this, "ownGoalAreaRadius", [] {})
  , opponentGoalAreaRadius_(*this, "opponentGoalAreaRadius", [] {})
  , useInWalkKickAsStrongDribble_(*this, "useInWalkKickAsStrongDribble", [] {})
  , useInWalkKickInKickOff_(*this, "useInWalkKickInKickOff", [] {})
  , useInWalkKickToClearBall_(*this, "useInWalkKickToClearBall", [] {})
  , useInWalkKickToClearBallASAP_(*this, "useInWalkKickToClearBallASAP", [] {})
  , useInWalkKickToScoreGoal_(*this, "useInWalkKickToScoreGoal", [] {})
  , useOnlyThisFoot_(*this, "useOnlyThisFoot", [] {})
  , useSideKickParam_(*this, "useSideKick", [] {})
  , useStrongDribble_(*this, "useStrongDribble", [] {})
  , useTurnKickParam_(*this, "useTurnKick", [] {})
  , forceKick_(*this, "forceKick", [] {})
  , ballState_(*this)
  , collisionDetectorData_(*this)
  , cycleInfo_(*this)
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , kickConfigurationData_(*this)
  , teamObstacleData_(*this)
  , robotPosition_(*this)
  , setPosition_(*this)
  , teamBallModel_(*this)
  , teamPlayers_(*this)
  , worldState_(*this)
  , lastAction_(StrikerAction::Type::DRIBBLE)
  , lastIsBallNearOpponentGoal_(false)
  , lastIsBallNearOwnGoal_(false)
  , lastSign_(useOnlyThisFoot_())
  , lastPassTarget_(0)
  , strikerAction_(*this)
{
  angleToBallDribble_() *= TO_RAD;
  angleToBallKick_() *= TO_RAD;
  asapDeviationAngle_() *= TO_RAD;
  kickOpeningAngle_() *= TO_RAD;
  assert(kickOpeningAngle_() < 180.f * TO_RAD);
}

void StrikerActionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");
  if ((gameControllerState_->gameState != GameState::PLAYING &&
       gameControllerState_->gameState != GameState::READY &&
       gameControllerState_->gameState != GameState::SET) ||
      gameControllerState_->penalty != Penalty::NONE ||
      gameControllerState_->gamePhase != GamePhase::NORMAL ||
      gameControllerState_->setPlay == SetPlay::CORNER_KICK ||
      teamBallModel_->ballType == TeamBallModel::BallType::NONE)
  {
    return;
  }

  // there has to be a valid reference to a last sign for kick pose evaluation
  int useOnlyThisFoot = useOnlyThisFoot_();
  const bool forceSign = useOnlyThisFoot_() != 0;
  int& lastSign = forceSign ? useOnlyThisFoot : lastSign_;

  if (forceKick_())
  {
    const Vector2f absOpponentGoal(fieldDimensions_->fieldLength / 2.0f, 0.0f);
    createStrikerAction(KickType::FORWARD, absOpponentGoal, lastSign, forceSign);
    return;
  }

  // do kickoff dribbling?
  if (worldState_->ballInCenterCircle && gameControllerState_->kickingTeam &&
      gameControllerState_->gameState == GameState::PLAYING &&
      cycleInfo_->getAbsoluteTimeDifference(gameControllerState_->gameStateChanged) < 10s)
  {
    // if the ball is in center circle and we have kickoff and the game state has recently changed
    // to playing try to dribble around the opponent. The vector from set position to field center
    // dictates the dribble direction at kickoff if set position is valid. Otherwise the
    // interpolated dribble target is used.
    const Vector2f interpolatedBallTarget =
        teamBallModel_->absPosition + (getInterpolatedDirection().normalized() * 10.f);
    const Vector2f dribbleOffsetTarget =
        setPosition_->valid
            ? fieldDimensions_->fieldLength / 2.f * (-1.f * setPosition_->position).normalized()
            : interpolatedBallTarget;

    strikerAction_->target = dribbleOffsetTarget;

    if (useInWalkKickInKickOff_())
    {
      createStrikerAction(KickType::FORWARD, dribbleOffsetTarget, lastSign, forceSign);
      return;
    }
    createStrikerAction(dribbleOffsetTarget, lastSign, forceSign);
    return;
  }

  // is ball near to own goal?
  if (isBallNearOwnGoal())
  {
    if (worldState_->ballInGoalBoxArea)
    {
      const Vector2f interpolatedDirection(getInterpolatedDirection());
      const Vector2f robotToBall(teamBallModel_->absPosition - robotPosition_->pose.position());
      const float currentAngleToInterpolatedDirection(std::atan2(
          interpolatedDirection.x() * robotToBall.y() - interpolatedDirection.y() * robotToBall.x(),
          interpolatedDirection.dot(robotToBall))); // atan2(det, dot)

      const float clippedAngle(Range<float>::clipToGivenRange(currentAngleToInterpolatedDirection,
                                                              -1.f * asapDeviationAngle_(),
                                                              asapDeviationAngle_()));
      const Vector2f clippedDirection(Rotation2Df(clippedAngle) * interpolatedDirection);
      const Vector2f ballTarget(teamBallModel_->absPosition + clippedDirection.normalized() * 2.5f);

      if (isGivenWayFree(clippedDirection) && kickAwayFromGoal_() && !amIAlone())
      {
        if (useSideKickParam_() && useSideKick())
        {
          // side kick away from goal
          assert(false);
          return;
        }
        else
        {
          // shoot away from goal ASAP
          strikerAction_->target = ballTarget;
          if (useInWalkKickToClearBallASAP_())
          {
            createStrikerAction(InWalkKickType::FORWARD, ballTarget);
            return;
          }
          else
          {
            createStrikerAction(KickType::FORWARD, ballTarget, lastSign, forceSign);
            return;
          }
        }
      }
      else
      {
        if (useTurnKickParam_() && useTurnKick())
        {
          // turn kick
          assert(false);
          return;
        }
        else
        {
          // dribble in interpolated direction
          createStrikerAction(ballTarget, lastSign, forceSign);
          return;
        }
      }
    }
    else
    {
      if (isInterpolatedWayFree() && kickAwayFromGoal_() && !amIAlone())
      {
        if (useSideKickParam_() && useSideKick())
        {
          // side kick away from goal
          assert(false);
          return;
        }
        else
        {
          // shoot away from goal
          const Vector2f ballTarget(teamBallModel_->absPosition +
                                    getInterpolatedDirection().normalized() * 2.5f);
          if (useInWalkKickToClearBall_())
          {
            createStrikerAction(InWalkKickType::FORWARD, ballTarget);
            return;
          }
          else
          {
            createStrikerAction(KickType::FORWARD, ballTarget, lastSign, forceSign);
            return;
          }
        }
      }
      else
      {
        if (useTurnKickParam_() && useTurnKick())
        {
          // turn kick
          assert(false);
          return;
        }
        else
        {
          // dribble in interpolated direction
          const Vector2f ballTarget =
              teamBallModel_->absPosition + (getInterpolatedDirection().normalized() * 10.f);
          createStrikerAction(ballTarget, lastSign, forceSign);
          return;
        }
      }
    }
  }
  else if (isBallNearOpponentGoal())
  {
    if (worldState_->ballInGoalBoxArea)
    {
      // use side kick?
      if (useSideKickParam_() && useSideKick())
      {
        // side kick into goal
        assert(false);
        return;
      }
      else
      {
        // dribble fast into goal (ASAP)
        Vector2f ballTarget = Vector2f(fieldDimensions_->fieldLength / 2.f, 0.f);

        const Vector2f robotToBall = teamBallModel_->absPosition - robotPosition_->pose.position();
        const float angleRobotToBall = std::atan2(robotToBall.y(), robotToBall.x());
        const float dribbleAngle =
            (worldState_->ballIsToMyLeft ? 1.f : -1.f) *
            Range<float>::clipToGivenRange(std::abs(angleRobotToBall), 0.f, 30.f * TO_RAD);
        const Line<float> dribbleLine = Line<float>(
            teamBallModel_->absPosition,
            teamBallModel_->absPosition + Vector2f(std::cos(dribbleAngle), std::sin(dribbleAngle)));
        ballTarget = Vector2f(
            ballTarget.x(), Range<float>::clipToGivenRange(dribbleLine.getY(ballTarget.x()),
                                                           -fieldDimensions_->goalInnerWidth / 3.f,
                                                           fieldDimensions_->goalInnerWidth / 3.f));
        createStrikerAction(ballTarget, lastSign, forceSign);
        return;
      }
    }
    else
    {
      const Vector2f goalCenter{fieldDimensions_->fieldLength / 2.f, 0.f};
      const Vector2f goalLeft{goalCenter + Vector2f{0.f, fieldDimensions_->goalInnerWidth / 4.f}};
      const Vector2f goalRight{goalCenter - Vector2f{0.f, fieldDimensions_->goalInnerWidth / 4.f}};
      const auto kickTarget{checkForBestKickTarget(goalLeft, goalCenter, goalRight)};
      if (kickIntoGoal_() && kickTarget != KickTarget::NONE)
      {
        // shoot into goal
        const Vector2f ballTarget{kickTarget == KickTarget::CENTER
                                      ? goalCenter
                                      : (kickTarget == KickTarget::LEFT ? goalLeft : goalRight)};
        if (useInWalkKickToScoreGoal_())
        {
          createStrikerAction(InWalkKickType::FORWARD, ballTarget);
          return;
        }
        else
        {
          createStrikerAction(KickType::FORWARD, ballTarget, lastSign, forceSign);
          return;
        }
      }
      else
      {
        // dribble in interpolated direction
        const Vector2f ballTarget =
            teamBallModel_->absPosition + (getInterpolatedDirection().normalized() * 10.f);
        createStrikerAction(ballTarget, lastSign, forceSign);
        return;
      }
    }
  }
  else
  {
    if (isInterpolatedWayFree() && useStrongDribble_())
    {
      // strong dribble
      const Vector2f ballTarget =
          teamBallModel_->absPosition + (getInterpolatedDirection().normalized() * 10.f);

      if (useInWalkKickAsStrongDribble_())
      {
        createStrikerAction(InWalkKickType::FORWARD, ballTarget);
        return;
      }
      else
      {
        createStrikerAction(ballTarget, lastSign, forceSign);
        return;
      }
    }
    else
    {
      if (useTurnKickParam_() && useTurnKick())
      {
        // turn kick
        assert(false);
      }
      else
      {
        // dribble in interpolated direction
        const Vector2f ballTarget =
            teamBallModel_->absPosition + (getInterpolatedDirection().normalized() * 10.f);
        createStrikerAction(ballTarget, lastSign, forceSign);
        return;
      }
    }
  }
}

void StrikerActionProvider::createStrikerAction(const KickType kickType, const Vector2f& absTarget,
                                                int& lastSign, const bool forceSign)
{
  const auto kick = kickConfigurationData_->kicks[kickType];

  strikerAction_->type = StrikerAction::Type::KICK;
  strikerAction_->kickType = kickType;
  strikerAction_->target = absTarget;
  strikerAction_->kickPose =
      BallUtils::kickPose(teamBallModel_->relPosition, robotPosition_->fieldToRobot(absTarget),
                          kick.distanceToBall.x(), lastSign, forceSign, kick.distanceToBall.y());
  strikerAction_->kickable =
      BallUtils::kickable(strikerAction_->kickPose, *ballState_, kick.distanceToBall.x(),
                          angleToBallKick_(), kick.distanceToBall.y(), strikerAction_->kickable);
  strikerAction_->valid = true;
}

void StrikerActionProvider::createStrikerAction(const Vector2f& absTarget, int& lastSign,
                                                const bool forceSign)
{
  strikerAction_->type = StrikerAction::Type::DRIBBLE;
  strikerAction_->target = absTarget;
  strikerAction_->kickPose = BallUtils::kickPose(
      teamBallModel_->relPosition, robotPosition_->fieldToRobot(absTarget),
      distanceToBallDribble_().x(), lastSign, forceSign, distanceToBallDribble_().y());
  strikerAction_->kickable = BallUtils::kickable(
      strikerAction_->kickPose, *ballState_, distanceToBallDribble_().x(), angleToBallDribble_(),
      distanceToBallDribble_().y(), strikerAction_->kickable);
  strikerAction_->valid = true;
}


void StrikerActionProvider::createStrikerAction(const InWalkKickType inWalkKickType,
                                                const Vector2f& absTarget)
{
  const auto inWalkKick = kickConfigurationData_->inWalkKicks[inWalkKickType];
  const auto kickFoot = KickFoot::LEFT;

  strikerAction_->type = StrikerAction::Type::IN_WALK_KICK;
  strikerAction_->inWalkKickType = InWalkKickType::FORWARD;
  strikerAction_->target = absTarget;
  strikerAction_->kickPose = BallUtils::kickPose(inWalkKick, kickFoot, teamBallModel_->relPosition,
                                                 robotPosition_->fieldToRobot(absTarget));
  strikerAction_->kickable =
      BallUtils::kickable(strikerAction_->kickPose, inWalkKick, kickFoot, *ballState_,
                          angleToBallKick_(), strikerAction_->kickable);
  strikerAction_->valid = true;
}

Vector2f StrikerActionProvider::getInterpolatedDirection() const
{
  Vector2f interpolatedDirection = Vector2f::Zero();
  const Vector2f fieldSize(fieldDimensions_->fieldLength / 2.f, fieldDimensions_->fieldWidth / 2.f);
  for (const auto& it : dribbleMapInterpolationPoints_())
  {
    // squared to increase the influence of near dribbleMapInterpolationPoints
    const float distance = (Vector2f(it[0].x() * fieldSize.x(), it[0].y() * fieldSize.y()) -
                            teamBallModel_->absPosition)
                               .squaredNorm();
    interpolatedDirection += it[1].normalized() / (1.f + distance);
  }
  return interpolatedDirection;
}

float StrikerActionProvider::rateKick(const Vector2f& kickTarget, Vector2f leftClipPoint,
                                      Vector2f rightClipPoint)
{
  Vector2f ballToLeftClipPoint(leftClipPoint - teamBallModel_->absPosition);
  Vector2f ballToRightClipPoint(rightClipPoint - teamBallModel_->absPosition);

  // swap clipping points if the left point is right of the right point. It is obviously not the
  // left point then.
  if (ballToLeftClipPoint.x() * ballToRightClipPoint.y() -
          ballToLeftClipPoint.y() * ballToRightClipPoint.x() >
      0.f)
  {
    const Vector2f swap(leftClipPoint);
    leftClipPoint = rightClipPoint;
    rightClipPoint = swap;
    ballToLeftClipPoint = leftClipPoint - teamBallModel_->absPosition;
    ballToRightClipPoint = rightClipPoint - teamBallModel_->absPosition;
  }

  const auto chunkCount = static_cast<int>(kickRatingChunkWeights_().size());
  assert(chunkCount > 0);
  const float angleStepSize = kickOpeningAngle_() / static_cast<float>(chunkCount - 1);
  const Vector2f ballToKickTarget = kickTarget - teamBallModel_->absPosition;
  const Line<float> chunkLine(leftClipPoint, rightClipPoint);

  std::vector<bool> kickRatingChunks(static_cast<size_t>(chunkCount), true);
  std::vector<Vector2f> hitPoints(static_cast<size_t>(chunkCount), Vector2f(1337.f, 1337.f));

  for (int i = 0; i < chunkCount; i++)
  {
    const Vector2f currentVector(Rotation2Df(-kickOpeningAngle_() / 2.f + i * angleStepSize) *
                                 ballToKickTarget);
    // if the current vector is between the left and the right clipping point. Then it is blocked by
    // the current obstacle.
    if (currentVector.x() * ballToLeftClipPoint.y() - currentVector.y() * ballToLeftClipPoint.x() <
            0 ||
        currentVector.x() * ballToRightClipPoint.y() -
                currentVector.y() * ballToRightClipPoint.x() >
            0)
    {
      kickRatingChunks[i] = false;
    }
  }

  for (int i = 0; i < chunkCount; i++)
  {
    if (kickRatingChunks[i])
    {
      const Line<float> lawnSprinklerLine(
          teamBallModel_->absPosition,
          teamBallModel_->absPosition +
              Rotation2Df(-kickOpeningAngle_() / 2.f + i * angleStepSize) * ballToKickTarget);
      if (!Geometry::getIntersection(lawnSprinklerLine, chunkLine, hitPoints[i]))
      {
        Log<M_BRAIN>(LogLevel::ERROR) << "Rate kick failed, no intersection between "
                                         "lawnSprinklerLine and chunkLine";
        assert(false);
        return 0.f;
      }
    }
  }

  for (const auto& obstacle : teamObstacleData_->obstacles)
  {
    if ((obstacle.type == ObstacleType::FALLEN_ANONYMOUS_ROBOT ||
         obstacle.type == ObstacleType::FALLEN_HOSTILE_ROBOT ||
         obstacle.type == ObstacleType::FALLEN_TEAM_ROBOT ||
         obstacle.type == ObstacleType::HOSTILE_ROBOT ||
         obstacle.type == ObstacleType::ANONYMOUS_ROBOT ||
         obstacle.type == ObstacleType::TEAM_ROBOT || obstacle.type == ObstacleType::GOAL_POST ||
         obstacle.type == ObstacleType::UNKNOWN))
    {
      if ((obstacle.absolutePosition - teamBallModel_->absPosition).norm() <= obstacle.radius)
      {
        // ball inside obstacle --> do not kick
        kickRatingChunks = std::vector<bool>(static_cast<size_t>(chunkCount), false);
        break;
      }
      if (ballToKickTarget.squaredNorm() > (kickTarget - obstacle.absolutePosition).squaredNorm())
      {
        std::pair<Vector2f, Vector2f> tangentPoints;
        if (Geometry::getTangentPointsOfCircle(
                teamBallModel_->absPosition, obstacle.absolutePosition,
                obstacle.radius + fieldDimensions_->ballDiameter / 2.f, tangentPoints))
        {
          debug().update(mount_ + ".firstShadowPoint", tangentPoints.first);
          debug().update(mount_ + ".secondShadowPoint", tangentPoints.second);
          const std::pair<Vector2f, Vector2f> ballToTangentPoints(
              tangentPoints.first - teamBallModel_->absPosition,
              tangentPoints.second - teamBallModel_->absPosition);
          if (ballToTangentPoints.first.x() * ballToLeftClipPoint.y() -
                  ballToTangentPoints.first.y() * ballToLeftClipPoint.x() <
              0.f)
          {
            // clip first tangentPoint to leftGoalPost
            tangentPoints.first = leftClipPoint;
          }
          else if (ballToTangentPoints.first.x() * ballToRightClipPoint.y() -
                       ballToTangentPoints.first.y() * ballToRightClipPoint.x() >
                   0.f)
          {
            // clip first tangentPoint to rightGoalPost
            tangentPoints.first = rightClipPoint;
          }
          if (ballToTangentPoints.second.x() * ballToLeftClipPoint.y() -
                  ballToTangentPoints.second.y() * ballToLeftClipPoint.x() <
              0.f)
          {
            // clip second tangentPoint to leftGoalPost
            tangentPoints.second = leftClipPoint;
          }
          else if (ballToTangentPoints.second.x() * ballToRightClipPoint.y() -
                       ballToTangentPoints.second.y() * ballToRightClipPoint.x() >
                   0.f)
          {
            // clip second tangentPoint to rightGoalPost
            tangentPoints.second = rightClipPoint;
          }
          Vector2f firstShadowPoint;
          const Line<float> ballToFirstTangentPoint(teamBallModel_->absPosition,
                                                    tangentPoints.first);
          if (!Geometry::getIntersection(ballToFirstTangentPoint, chunkLine, firstShadowPoint))
          {
            Log<M_BRAIN>(LogLevel::ERROR) << "Rate kick failed, no intersection between "
                                             "ballToFirstTangentPoint and chunkLine";
            assert(false);
            return 0.f;
          }

          Vector2f secondShadowPoint;
          const Line<float> ballToSecondTangentPoint(teamBallModel_->absPosition,
                                                     tangentPoints.second);
          if (!Geometry::getIntersection(ballToSecondTangentPoint, chunkLine, secondShadowPoint))
          {
            Log<M_BRAIN>(LogLevel::ERROR) << "Rate kick failed, no intersection between "
                                             "ballToSecondTangentPoint and chunkLine";
            assert(false);
            return 0.f;
          }

          debug().update(mount_ + ".firstShadowPointAfter", firstShadowPoint);
          debug().update(mount_ + ".secondShadowPointAfter", secondShadowPoint);

          for (int i = 0; i < chunkCount; i++)
          {
            if (!kickRatingChunks[i])
            {
              continue;
            }
            if ((firstShadowPoint - hitPoints[i]).dot(secondShadowPoint - hitPoints[i]) < 0.f)
            {
              kickRatingChunks[i] = false;
            }
          }
        }
      }
    }
  }

  float sumAllChunks = 0.f;
  float sumFreeChunks = 0.f;
  for (int i = 0; i < chunkCount; i++)
  {
    sumAllChunks += kickRatingChunkWeights_()[i];
    sumFreeChunks += (kickRatingChunks[i] ? kickRatingChunkWeights_()[i] : 0.f);
  }
  assert(sumAllChunks != 0); // do not divide by zero
  const float currentKickChance = sumFreeChunks / sumAllChunks;
  return currentKickChance;
}

bool StrikerActionProvider::collisionDetected() const
{
  return (collisionDetectorData_->collisionLeft || collisionDetectorData_->collisionRight ||
          collisionDetectorData_->duelRigid);
}

StrikerActionProvider::KickTarget
StrikerActionProvider::checkForBestKickTarget(const Vector2f& left, const Vector2f& center,
                                              const Vector2f& right)
{
  if (collisionDetected())
  {
    lastKickTarget_ = KickTarget::NONE;
    return KickTarget::NONE;
  }
  const Vector2f leftGoalPost{fieldDimensions_->fieldLength / 2,
                              fieldDimensions_->goalInnerWidth / 2};
  const Vector2f rightGoalPost{fieldDimensions_->fieldLength / 2,
                               -fieldDimensions_->goalInnerWidth / 2};
  const float leftRating{rateKick(left, leftGoalPost, rightGoalPost)};
  const float centerRating{rateKick(center, leftGoalPost, rightGoalPost)};
  const float rightRating{rateKick(right, leftGoalPost, rightGoalPost)};
  const float maxRating{std::max({centerRating, leftRating, rightRating})};
  KickTarget kickTarget;
  if (Hysteresis::smallerThan(maxRating, kickRatingThreshold_(), 0.05f,
                              lastKickTarget_ == KickTarget::NONE))
  {
    kickTarget = KickTarget::NONE;
  }
  else if (leftRating > centerRating && leftRating > rightRating)
  {
    kickTarget = KickTarget::LEFT;
  }
  else if (rightRating > centerRating && rightRating > leftRating)
  {
    kickTarget = KickTarget::RIGHT;
  }
  else
  {
    kickTarget = KickTarget::CENTER;
  }
  lastKickTarget_ = kickTarget;
  return kickTarget;
}

bool StrikerActionProvider::isInterpolatedWayFree()
{
  const Vector2f interpolatedDirection(getInterpolatedDirection().normalized() * 2.5f);
  return isGivenWayFree(interpolatedDirection);
}

bool StrikerActionProvider::isGivenWayFree(const Vector2f& direction)
{
  if (collisionDetected())
  {
    wasGivenWayFreeLastCycle_ = false;
    return false;
  }
  const Vector2f kickTarget(teamBallModel_->absPosition + direction);
  const Vector2f leftClipPoint(teamBallModel_->absPosition +
                               Rotation2Df(kickOpeningAngle_() / 2.f) * direction /
                                   std::cos(kickOpeningAngle_() / 2.f));
  const Vector2f rightClipPoint(teamBallModel_->absPosition +
                                Rotation2Df(-kickOpeningAngle_() / 2.f) * direction /
                                    std::cos(kickOpeningAngle_() / 2.f));
  const bool isGivenWayFree{
      Hysteresis::greaterThan(rateKick(kickTarget, leftClipPoint, rightClipPoint),
                              kickRatingThreshold_(), 0.1f, wasGivenWayFreeLastCycle_)};
  wasGivenWayFreeLastCycle_ = isGivenWayFree;
  return isGivenWayFree;
}

bool StrikerActionProvider::amIAlone() const
{
  for (const auto& teamPlayer : teamPlayers_->players)
  {
    if (!teamPlayer.penalized)
    {
      return false;
    }
  }
  return true;
}

bool StrikerActionProvider::isBallNearOwnGoal()
{
  const float ballDistanceToOwnGoal =
      (Vector2f(-fieldDimensions_->fieldLength / 2.f, 0.f) - teamBallModel_->absPosition).norm();
  lastIsBallNearOwnGoal_ = Hysteresis::smallerThan(ballDistanceToOwnGoal, ownGoalAreaRadius_(),
                                                   0.25f, lastIsBallNearOwnGoal_);
  return (lastIsBallNearOwnGoal_);
}

bool StrikerActionProvider::isBallNearOpponentGoal()
{
  const float ballDistanceToOpponentGoal =
      (Vector2f(fieldDimensions_->fieldLength / 2.f, 0.f) - teamBallModel_->absPosition).norm();
  lastIsBallNearOpponentGoal_ = Hysteresis::smallerThan(
      ballDistanceToOpponentGoal, opponentGoalAreaRadius_(), 0.25f, lastIsBallNearOpponentGoal_);
  return (lastIsBallNearOpponentGoal_);
}

bool StrikerActionProvider::useSideKick() const
{
  return false;
}

bool StrikerActionProvider::useTurnKick() const
{
  return false;
}

float StrikerActionProvider::ratePosition(const Vector2f& position) const
{
  const Vector2f goalPosition(fieldDimensions_->fieldLength / 2.f, 0.f);
  const Vector2f positionToGoal = goalPosition - position;

  const float distanceToGoal = positionToGoal.norm();
  const float goalAngle = std::abs(std::atan2(positionToGoal.y(), positionToGoal.x()));
  return distanceToGoal + goalAngle * 0.75f;
}

StrikerActionProvider::PassTarget
StrikerActionProvider::findPassTarget(const float ballRating) const
{
  PassTarget passTarget;
  passTarget.number = 0;
  passTarget.rating = ballRating - lastTargetBonus_; // treat the ball rating like a lastTarget
  for (const auto& player : teamPlayers_->players)
  {
    if (player.penalized || player.fallen)
    {
      continue;
    }
    const float playerDistance = (player.pose.position() - teamBallModel_->absPosition).norm();
    if (playerDistance < 1.5f || playerDistance > 3.f)
    {
      continue;
    }
    const float bonus =
        (lastAction_ == StrikerAction::Type::PASS && player.playerNumber == lastPassTarget_)
            ? lastTargetBonus_
            : 0.f;
    const float playerRating = ratePosition(player.pose.position()) - bonus;
    if (playerRating >= passTarget.rating)
    {
      continue;
    }
    passTarget.number = player.playerNumber;
    passTarget.rating = playerRating;
    passTarget.position = player.pose.position();
  }
  return passTarget;
}
