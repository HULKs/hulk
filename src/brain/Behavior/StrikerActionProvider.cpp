#include <cmath>

#include "Tools/BallUtils.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Hysteresis.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/Math/Random.hpp"
#include "Tools/Math/Range.hpp"

#include "print.h"

#include "StrikerActionProvider.hpp"


StrikerActionProvider::StrikerActionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , angleToBallDribble_(*this, "angleToBallDribble", [this] { angleToBallDribble_() *= TO_RAD; })
  , angleToBallKick_(*this, "angleToBallKick", [this] { angleToBallKick_() *= TO_RAD; })
  , asapDeviationAngle_(*this, "asapDeviationAngle", [this] { asapDeviationAngle_() *= TO_RAD; })
  , distanceToBallDribble_(*this, "distanceToBallDribble", [] {})
  , distanceToBallKick_(*this, "distanceToBallKick", [] {})
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
  , useOnlyThisFoot_(*this, "useOnlyThisFoot", [] {})
  , useSideKickParam_(*this, "useSideKick", [] {})
  , useStrongDribble_(*this, "useStrongDribble", [] {})
  , useTurnKickParam_(*this, "useTurnKick", [] {})
  , ballState_(*this)
  , collisionDetectorData_(*this)
  , cycleInfo_(*this)
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , kickConfigurationData_(*this)
  , obstacleData_(*this)
  , robotPosition_(*this)
  , setPosition_(*this)
  , teamBallModel_(*this)
  , teamPlayers_(*this)
  , worldState_(*this)
  , lastAction_(StrikerAction::DRIBBLE)
  , lastIsBallNearOpponentGoal_(false)
  , lastIsBallNearOwnGoal_(false)
  , lastKickRating_(false)
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
      teamBallModel_->ballType == TeamBallModel::BallType::NONE)
  {
    return;
  }

  const Vector2f absBallPosition = teamBallModel_->position;
  const Vector2f relBallPosition = robotPosition_->fieldToRobot(absBallPosition);
  // there has to be a valid reference to a last sign for kick pose evaluation
  int useOnlyThisFoot = useOnlyThisFoot_();
  const bool forceSign = useOnlyThisFoot_() != 0;
  int& lastSign = forceSign ? useOnlyThisFoot : lastSign_;

  // do kickoff dribbling?
  if (worldState_->ballInCenterCircle && gameControllerState_->kickingTeam &&
      gameControllerState_->gameState == GameState::PLAYING &&
      cycleInfo_->getTimeDiff(gameControllerState_->gameStateChanged) < 10.f)
  {
    // if the ball is in center circle and we have kickoff and the game state has recently changed
    // to playing try to dribble around the opponent. The vector from set position to field center
    // dictates the dribble direction at kickoff if set position is valid. Otherwise the
    // interpolated dribble target is used.
    const Vector2f interpolatedBallTarget =
        absBallPosition + (getInterpolatedDirection().normalized() * 10.f);
    const Vector2f dribbleOffsetTarget =
        setPosition_->valid
            ? fieldDimensions_->fieldLength / 2.f * (-1.f * setPosition_->position).normalized()
            : interpolatedBallTarget;

    strikerAction_->type = StrikerAction::Type::DRIBBLE;
    strikerAction_->target = dribbleOffsetTarget;
    strikerAction_->kickType = StrikerAction::KickType::NONE;
    strikerAction_->kickPose =
        BallUtils::kickPose(relBallPosition, robotPosition_->fieldToRobot(dribbleOffsetTarget),
                            distanceToBallDribble_().x(), lastSign, forceSign, distanceToBallDribble_().y());
    strikerAction_->kickable =
        BallUtils::kickable(strikerAction_->kickPose, *ballState_, distanceToBallDribble_().x(),
                            angleToBallDribble_(), distanceToBallDribble_().y(), strikerAction_->kickable);
    strikerAction_->valid = true;
    return;
  }

  // is ball near to own goal?
  if (isBallNearOwnGoal())
  {
    if (worldState_->ballInPenaltyArea)
    {
      const Vector2f interpolatedDirection(getInterpolatedDirection());
      const Vector2f robotToBall(absBallPosition - robotPosition_->pose.position);
      const float currentAngleToInterpolatedDirection(std::atan2(
          interpolatedDirection.x() * robotToBall.y() - interpolatedDirection.y() * robotToBall.x(),
          interpolatedDirection.dot(robotToBall))); // atan2(det, dot)

      const float clippedAngle(Range<float>::clipToGivenRange(
          currentAngleToInterpolatedDirection, -1.f * asapDeviationAngle_(), asapDeviationAngle_()));
      const Vector2f clippedDirection(Rotation2Df(clippedAngle) * interpolatedDirection);
      const Vector2f ballTarget(teamBallModel_->position + clippedDirection.normalized() * 2.5f);

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
          strikerAction_->type = StrikerAction::Type::KICK_INTO_GOAL;
          strikerAction_->target = ballTarget;
          strikerAction_->kickType = StrikerAction::KickType::FORWARD;
          strikerAction_->kickPose = BallUtils::kickPose(
              relBallPosition, robotPosition_->fieldToRobot(ballTarget), distanceToBallKick_().x(),
              lastSign, forceSign, distanceToBallKick_().y());
          strikerAction_->kickable = BallUtils::kickable(
              strikerAction_->kickPose, *ballState_, distanceToBallKick_().x(), angleToBallKick_(),
              distanceToBallKick_().y(), strikerAction_->kickable);
          strikerAction_->valid = true;
          return;
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
          strikerAction_->type = StrikerAction::Type::DRIBBLE;
          strikerAction_->target = ballTarget;
          strikerAction_->kickType = StrikerAction::KickType::NONE;
          strikerAction_->kickPose =
              BallUtils::kickPose(relBallPosition, robotPosition_->fieldToRobot(ballTarget),
                                  distanceToBallDribble_().x(), lastSign, forceSign, distanceToBallDribble_().y());
          strikerAction_->kickable =
              BallUtils::kickable(strikerAction_->kickPose, *ballState_, distanceToBallDribble_().x(),
                                  angleToBallDribble_(), distanceToBallDribble_().y(), strikerAction_->kickable);
          strikerAction_->valid = true;
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
          const Vector2f ballTarget(teamBallModel_->position +
                                    getInterpolatedDirection().normalized() * 2.5f);
          // fill data
          strikerAction_->type = StrikerAction::Type::KICK_INTO_GOAL;
          strikerAction_->target = ballTarget;
          strikerAction_->kickType = StrikerAction::KickType::FORWARD;
          strikerAction_->kickPose = BallUtils::kickPose(
              relBallPosition, robotPosition_->fieldToRobot(ballTarget), distanceToBallKick_().x(),
              lastSign, forceSign, distanceToBallKick_().y());
          strikerAction_->kickable = BallUtils::kickable(
              strikerAction_->kickPose, *ballState_, distanceToBallKick_().x(), angleToBallKick_(),
              distanceToBallKick_().y(), strikerAction_->kickable);
          strikerAction_->valid = true;
          return;
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
              absBallPosition + (getInterpolatedDirection().normalized() * 10.f);

          // fill data
          strikerAction_->type = StrikerAction::Type::DRIBBLE;
          strikerAction_->target = ballTarget;
          strikerAction_->kickType = StrikerAction::KickType::NONE;
          strikerAction_->kickPose =
              BallUtils::kickPose(relBallPosition, robotPosition_->fieldToRobot(ballTarget),
                                  distanceToBallDribble_().x(), lastSign, forceSign, distanceToBallDribble_().y());
          strikerAction_->kickable =
              BallUtils::kickable(strikerAction_->kickPose, *ballState_, distanceToBallDribble_().x(),
                                  angleToBallDribble_(), distanceToBallDribble_().y(), strikerAction_->kickable);
          strikerAction_->valid = true;
          return;
        }
      }
    }
  }
  else if (isBallNearOpponentGoal())
  {
    if (worldState_->ballInPenaltyArea)
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

        const Vector2f robotToBall = absBallPosition - robotPosition_->pose.position;
        const float angleRobotToBall = std::atan2(robotToBall.y(), robotToBall.x());
        const float dribbleAngle =
            (worldState_->ballIsToMyLeft ? 1.f : -1.f) *
            Range<float>::clipToGivenRange(std::abs(angleRobotToBall), 0.f, 30 * TO_RAD);
        const Line<float> dribbleLine =
            Line<float>(absBallPosition,
                        absBallPosition + Vector2f(std::cos(dribbleAngle), std::sin(dribbleAngle)));
        ballTarget = Vector2f(ballTarget.x(),
                              Range<float>::clipToGivenRange(dribbleLine.getY(ballTarget.x()),
                                                             -fieldDimensions_->goalInnerWidth / 3.f,
                                                             fieldDimensions_->goalInnerWidth / 3.f));

        // fill data
        strikerAction_->type = StrikerAction::Type::DRIBBLE_INTO_GOAL;
        strikerAction_->target = ballTarget;
        strikerAction_->kickType = StrikerAction::KickType::NONE;
        strikerAction_->kickPose =
            BallUtils::kickPose(relBallPosition, robotPosition_->fieldToRobot(ballTarget),
                                distanceToBallDribble_().x(), lastSign, forceSign, distanceToBallDribble_().y());
        strikerAction_->kickable =
            BallUtils::kickable(strikerAction_->kickPose, *ballState_, distanceToBallDribble_().x(),
                                angleToBallDribble_(), distanceToBallDribble_().y(), strikerAction_->kickable);
        strikerAction_->valid = true;
      }
    }
    else
    {
      if (isWayToGoalFree() && kickIntoGoal_())
      {
        // shoot into goal
        const Vector2f ballTarget(fieldDimensions_->fieldLength / 2.f, 0.f);
        // fill data
        strikerAction_->type = StrikerAction::Type::KICK_INTO_GOAL;
        strikerAction_->target = ballTarget;
        strikerAction_->kickType = StrikerAction::KickType::FORWARD;
        strikerAction_->kickPose = BallUtils::kickPose(
            relBallPosition, robotPosition_->fieldToRobot(ballTarget), distanceToBallKick_().x(),
            lastSign, forceSign, distanceToBallKick_().y());
        strikerAction_->kickable = BallUtils::kickable(
            strikerAction_->kickPose, *ballState_, distanceToBallKick_().x(), angleToBallKick_(),
            distanceToBallKick_().y(), strikerAction_->kickable);
        strikerAction_->valid = true;
        return;
      }
      else
      {
        // dribble in interpolated direction
        const Vector2f ballTarget =
            absBallPosition + (getInterpolatedDirection().normalized() * 10.f);

        // fill data
        strikerAction_->type = StrikerAction::Type::DRIBBLE;
        strikerAction_->target = ballTarget;
        strikerAction_->kickType = StrikerAction::KickType::NONE;
        strikerAction_->kickPose =
            BallUtils::kickPose(relBallPosition, robotPosition_->fieldToRobot(ballTarget),
                                distanceToBallDribble_().x(), lastSign, forceSign, distanceToBallDribble_().y());
        strikerAction_->kickable =
            BallUtils::kickable(strikerAction_->kickPose, *ballState_, distanceToBallDribble_().x(),
                                angleToBallDribble_(), distanceToBallDribble_().y(), strikerAction_->kickable);
        strikerAction_->valid = true;
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
          absBallPosition + (getInterpolatedDirection().normalized() * 10.f);

      // fill data
      strikerAction_->type = StrikerAction::Type::DRIBBLE;
      strikerAction_->target = ballTarget;
      strikerAction_->kickType = StrikerAction::KickType::NONE;
      strikerAction_->kickPose =
          BallUtils::kickPose(relBallPosition, robotPosition_->fieldToRobot(ballTarget),
                              distanceToBallDribble_().x(), lastSign, forceSign, distanceToBallDribble_().y());
      strikerAction_->kickable =
          BallUtils::kickable(strikerAction_->kickPose, *ballState_, distanceToBallDribble_().x(),
                              angleToBallDribble_(), distanceToBallDribble_().y(), strikerAction_->kickable);
      strikerAction_->valid = true;
      return;
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
            absBallPosition + (getInterpolatedDirection().normalized() * 10.f);

        // fill data
        strikerAction_->type = StrikerAction::Type::DRIBBLE;
        strikerAction_->target = ballTarget;
        strikerAction_->kickType = StrikerAction::KickType::NONE;
        strikerAction_->kickPose =
            BallUtils::kickPose(relBallPosition, robotPosition_->fieldToRobot(ballTarget),
                                distanceToBallDribble_().x(), lastSign, forceSign, distanceToBallDribble_().y());
        strikerAction_->kickable =
            BallUtils::kickable(strikerAction_->kickPose, *ballState_, distanceToBallDribble_().x(),
                                angleToBallDribble_(), distanceToBallDribble_().y(), strikerAction_->kickable);
        strikerAction_->valid = true;
        return;
      }
    }
  }
}

Vector2f StrikerActionProvider::getInterpolatedDirection() const
{
  const Vector2f ballPosition = teamBallModel_->position;
  Vector2f interpolatedDirection = Vector2f::Zero();
  const Vector2f fieldSize(fieldDimensions_->fieldLength / 2.f, fieldDimensions_->fieldWidth / 2.f);
  for (const auto& it : dribbleMapInterpolationPoints_())
  {
    // squared to increase the influence of near dribbleMapInterpolationPoints
    const float distance = (Vector2f(it[0].x() * fieldSize.x(), it[0].y() * fieldSize.y()) - ballPosition)
                         .squaredNorm();
    interpolatedDirection += it[1].normalized() / (1.f + distance);
  }
  return interpolatedDirection;
}

bool StrikerActionProvider::rateKick(const Vector2f& kickTarget, Vector2f leftClipPoint,
                                     Vector2f rightClipPoint)
{
  const Vector2f absBallPosition(teamBallModel_->position);
  Vector2f ballToLeftClipPoint(leftClipPoint - absBallPosition);
  Vector2f ballToRightClipPoint(rightClipPoint - absBallPosition);

  // swap clipping points if the left point is right of the right point. It is obviously not the
  // left point then.
  if (ballToLeftClipPoint.x() * ballToRightClipPoint.y() -
          ballToLeftClipPoint.y() * ballToRightClipPoint.x() > 0.f)
  {
    const Vector2f swap(leftClipPoint);
    leftClipPoint = rightClipPoint;
    rightClipPoint = swap;
    ballToLeftClipPoint = leftClipPoint - absBallPosition;
    ballToRightClipPoint = rightClipPoint - absBallPosition;
  }

  const auto chunkCount = static_cast<int>(kickRatingChunkWeights_().size());
  assert(chunkCount > 0);
  const float angleStepSize = kickOpeningAngle_() / static_cast<float>(chunkCount - 1);
  const Vector2f ballToKickTarget = kickTarget - absBallPosition;
  const Line<float> chunkLine(leftClipPoint, rightClipPoint);

  std::vector<bool> kickRatingChunks(static_cast<size_t>(chunkCount), true);
  std::vector<Vector2f> hitPoints(static_cast<size_t>(chunkCount), Vector2f(1337.f, 1337.f));

  for (int i = 0; i < chunkCount; i++)
  {
    const Vector2f currentVector(Rotation2Df(-kickOpeningAngle_() / 2.f + i * angleStepSize) * ballToKickTarget);
    // if the current vector is between the left and the right clipping point. Then it is blocked by
    // the current obstacle.
    if (currentVector.x() * ballToLeftClipPoint.y() - currentVector.y() * ballToLeftClipPoint.x() < 0 ||
        currentVector.x() * ballToRightClipPoint.y() - currentVector.y() * ballToRightClipPoint.x() > 0)
    {
      kickRatingChunks[i] = false;
    }
  }

  for (int i = 0; i < chunkCount; i++)
  {
    if (kickRatingChunks[i])
    {
      const Line<float> lawnSprinklerLine(absBallPosition,
                                    absBallPosition +
                                        Rotation2Df(-kickOpeningAngle_() / 2.f + i * angleStepSize) *
                                            ballToKickTarget);
      if (!Geometry::getIntersection(lawnSprinklerLine, chunkLine, hitPoints[i]))
      {
        Log(LogLevel::WARNING) << "Some meaningfull error message";
        assert(false);
      }
    }
  }

  for (auto& it : obstacleData_->obstacles)
  {
    const Vector2f absObstaclePosition = robotPosition_->robotToField(it.relativePosition);
    if ((it.type == ObstacleType::FALLEN_ANONYMOUS_ROBOT ||
         it.type == ObstacleType::FALLEN_HOSTILE_ROBOT ||
         it.type == ObstacleType::FALLEN_TEAM_ROBOT ||
         it.type == ObstacleType::HOSTILE_ROBOT ||
         it.type == ObstacleType::ANONYMOUS_ROBOT ||
         it.type == ObstacleType::TEAM_ROBOT ||
         it.type == ObstacleType::GOAL_POST ||
         it.type == ObstacleType::UNKNOWN))
    {
      if ((absObstaclePosition - absBallPosition).norm() <= it.radius)
      {
        // ball inside obstacle --> do not kick
        kickRatingChunks = std::vector<bool>(static_cast<size_t>(chunkCount), false);
        break;
      }
      else if (ballToKickTarget.squaredNorm() > (kickTarget - absObstaclePosition).squaredNorm())
      {
        std::pair<Vector2f, Vector2f> tangentPoints;
        if (Geometry::getTangentPointsOfCircle(absBallPosition, absObstaclePosition,
                                               it.radius + fieldDimensions_->ballDiameter / 2.f,
                                               tangentPoints))
        {
          debug().update(mount_ + ".firstShadowPoint", tangentPoints.first);
          debug().update(mount_ + ".secondShadowPoint", tangentPoints.second);
          const std::pair<Vector2f, Vector2f> ballToTangentPoints(
              tangentPoints.first - absBallPosition, tangentPoints.second - absBallPosition);
          if (ballToTangentPoints.first.x() * ballToLeftClipPoint.y() -
                  ballToTangentPoints.first.y() * ballToLeftClipPoint.x() < 0.f)
          {
            // clip first tangentPoint to leftGoalPost
            tangentPoints.first = leftClipPoint;
          }
          else if (ballToTangentPoints.first.x() * ballToRightClipPoint.y() -
                       ballToTangentPoints.first.y() * ballToRightClipPoint.x() > 0.f)
          {
            // clip first tangentPoint to rightGoalPost
            tangentPoints.first = rightClipPoint;
          }
          if (ballToTangentPoints.second.x() * ballToLeftClipPoint.y() -
                  ballToTangentPoints.second.y() * ballToLeftClipPoint.x() < 0.f)
          {
            // clip second tangentPoint to leftGoalPost
            tangentPoints.second = leftClipPoint;
          }
          else if (ballToTangentPoints.second.x() * ballToRightClipPoint.y() -
                       ballToTangentPoints.second.y() * ballToRightClipPoint.x() > 0.f)
          {
            // clip second tangentPoint to rightGoalPost
            tangentPoints.second = rightClipPoint;
          }
          Vector2f firstShadowPoint;
          const Line<float> ballToFirstTangentPoint(absBallPosition, tangentPoints.first);
          if (!Geometry::getIntersection(ballToFirstTangentPoint, chunkLine, firstShadowPoint))
          {
            Log(LogLevel::WARNING) << "Some meaningfull error message";
            assert(false);
          }

          Vector2f secondShadowPoint;
          const Line<float> ballToSecondTangentPoint(absBallPosition, tangentPoints.second);
          if (!Geometry::getIntersection(ballToSecondTangentPoint, chunkLine, secondShadowPoint))
          {
            Log(LogLevel::WARNING) << "Some meaningfull error message";
            assert(false);
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

  int sumAllChunks = 0;
  int sumFreeChunks = 0;
  for (int i = 0; i < chunkCount; i++)
  {
    sumAllChunks += kickRatingChunkWeights_()[i];
    sumFreeChunks += (kickRatingChunks[i] ? kickRatingChunkWeights_()[i] : 0);
  }
  const float currentKickChance = sumFreeChunks / static_cast<float>(sumAllChunks);

  const bool kickRating = lastKickRating_ ? currentKickChance > kickRatingThreshold_()[0]
                                    : currentKickChance > kickRatingThreshold_()[1];

  debug().update(mount_ + ".kickRatingChunks", kickRatingChunks);
  debug().update(mount_ + ".hitPoints", hitPoints);
  debug().update(mount_ + ".rateKick", kickRating);
  debug().update(mount_ + ".kickRatingChunkWeights", kickRatingChunkWeights_());

  lastKickRating_ = kickRating;
  return (kickRating);
}

bool StrikerActionProvider::collisionDetected() const
{
  return (collisionDetectorData_->collisionLeft || collisionDetectorData_->collisionRight ||
          collisionDetectorData_->duelRigid);
}

bool StrikerActionProvider::isWayToGoalFree()
{
  if (collisionDetected())
  {
    return false;
  }
  const Vector2f goalCenter(fieldDimensions_->fieldLength / 2.f, 0.f);
  const Vector2f leftGoalPost(fieldDimensions_->fieldLength / 2.f,
                              fieldDimensions_->goalInnerWidth / 2.f);
  const Vector2f rightGoalPost(fieldDimensions_->fieldLength / 2.f,
                               -fieldDimensions_->goalInnerWidth / 2.f);
  return rateKick(goalCenter, leftGoalPost, rightGoalPost);
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
    return false;
  }
  const Vector2f absBallPosition(teamBallModel_->position);
  const Vector2f kickTarget(absBallPosition + direction);
  const Vector2f leftClipPoint(absBallPosition + Rotation2Df(kickOpeningAngle_() / 2.f) * direction /
                                                     std::cos(kickOpeningAngle_() / 2.f));
  const Vector2f rightClipPoint(absBallPosition + Rotation2Df(-kickOpeningAngle_() / 2.f) *
                                                      direction /
                                                      std::cos(kickOpeningAngle_() / 2.f));
  return rateKick(kickTarget, leftClipPoint, rightClipPoint);
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
      (Vector2f(-fieldDimensions_->fieldLength / 2.f, 0.f) - teamBallModel_->position).norm();
  lastIsBallNearOwnGoal_ = Hysteresis<float>::smallerThan(
      ballDistanceToOwnGoal, ownGoalAreaRadius_(), 0.25f, lastIsBallNearOwnGoal_);
  return (lastIsBallNearOwnGoal_);
}

bool StrikerActionProvider::isBallNearOpponentGoal()
{
  const float ballDistanceToOpponentGoal =
      (Vector2f(fieldDimensions_->fieldLength / 2.f, 0.f) - teamBallModel_->position).norm();
  lastIsBallNearOpponentGoal_ = Hysteresis<float>::smallerThan(
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
    const float bonus =
        (lastAction_ == StrikerAction::PASS && player.playerNumber == lastPassTarget_)
            ? lastTargetBonus_
            : 0.f;
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
