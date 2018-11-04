#include <cassert>

#include "Tools/Chronometer.hpp"

#include "TeamBallFilter.hpp"


TeamBallFilter::TeamBallFilter(const ModuleManagerInterface& manager)
  : Module(manager)
  // TODO: The values in the config file are currently guessed.
  , maxAddAge_(*this, "maxAddAge", [] {})
  , minWaitAfterJumpToAddBall_(*this, "minWaitAfterJumpToAddBall", [] {})
  , maxBallVelocity_(*this, "maxBallVelocity", [] {})
  , minRemoveAge_(*this, "minRemoveAge", [] {})
  , maxCompatibilityDistance_(*this, "maxCompatibilityDistance", [] {})
  , insideFieldTolerance_(*this, "insideFieldTolerance", [] {})
  , playerConfiguration_(*this)
  , robotPosition_(*this)
  , ballState_(*this)
  , teamPlayers_(*this)
  , fieldDimensions_(*this)
  , cycleInfo_(*this)
  , gameControllerState_(*this)
  , teamBallModel_(*this)
{
}

void TeamBallFilter::addBallToBuffer(const unsigned int playerNumber, const Pose& pose,
                                     const Vector2f& relBallPosition,
                                     const Vector2f& relBallVelocity, const TimePoint timestamp)
{
  for (auto& ball : ballBuffer_)
  {
    if (ball.playerNumber == playerNumber)
    {
      ball.ball.position = pose * relBallPosition;
      ball.ball.velocity = pose.calculateGlobalOrientation(relBallVelocity);
      ball.distance = relBallPosition.norm();
      ball.timestamp = timestamp;
      return;
    }
  }
  TeamPlayerBall ball;
  ball.playerNumber = playerNumber;
  ball.ball.position = pose * relBallPosition;
  ball.ball.velocity = pose.calculateGlobalOrientation(relBallVelocity);
  ball.distance = relBallPosition.norm();
  ball.timestamp = timestamp;
  ballBuffer_.push_back(ball);
}

void TeamBallFilter::updateBallBuffer()
{
  // add team balls to buffer and calculate their distances from where their were seen
  for (auto& player : teamPlayers_->players)
  {
    // TODO: check other validity criteria
    if (!player.isPoseValid || player.penalized ||
        cycleInfo_->getTimeDiff(player.timeWhenBallWasSeen) > maxAddAge_() ||
        player.ballVelocity.norm() > maxBallVelocity_() ||
        cycleInfo_->getTimeDiff(player.timestampLastJumped) < minWaitAfterJumpToAddBall_())
    {
      continue;
    }
    addBallToBuffer(player.playerNumber, player.pose, player.ballPosition, player.ballVelocity,
                    player.timeWhenBallWasSeen);
  }
  // add own ball when found and confident like the team player balls
  if (ballState_->found && ballState_->confident)
  {
    addBallToBuffer(playerConfiguration_->playerNumber, robotPosition_->pose, ballState_->position,
                    ballState_->velocity, ballState_->timeWhenLastSeen);
  }
  else
  {
    // remove it when it is not found/confident anymore
    for (auto ball = ballBuffer_.begin(); ball != ballBuffer_.end(); ball++)
    {
      if (ball->playerNumber == playerConfiguration_->playerNumber)
      {
        ballBuffer_.erase(ball);
        break;
      }
    }
  }
  // remove those that are too old
  ballBuffer_.erase(std::remove_if(ballBuffer_.begin(), ballBuffer_.end(),
                                   [this](const TeamPlayerBall& ball) {
                                     return cycleInfo_->getTimeDiff(ball.timestamp) >
                                            minRemoveAge_();
                                   }),
                    ballBuffer_.end());
}

void TeamBallFilter::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  if ((gameControllerState_->gameState != GameState::PLAYING &&
       gameControllerState_->gameState != GameState::READY &&
       gameControllerState_->gameState != GameState::SET) ||
      gameControllerState_->penalty != Penalty::NONE)
  {
    ballBuffer_.clear();
    return;
  }
  // In READY, no balls are accepted but a ball on the kickoff spot is anticipated.
  if (gameControllerState_->gameState != GameState::READY)
  {
    updateBallBuffer();
    // Now try to establish a consensus on which ball is correct.
    BallCluster bestCluster;
    BallCluster currentCluster;
    for (auto& ball : ballBuffer_)
    {
      currentCluster.balls.resize(1);
      currentCluster.balls[0] = &ball;
      currentCluster.containsOwnBall = ball.playerNumber == playerConfiguration_->playerNumber;
      currentCluster.closestBallDistance = ball.distance;
      for (auto& ball2 : ballBuffer_)
      {
        if (ball.playerNumber == ball2.playerNumber)
        {
          continue;
        }
        if ((ball.ball.position - ball2.ball.position).norm() < maxCompatibilityDistance_())
        {
          if (ball.distance < currentCluster.closestBallDistance)
          {
            currentCluster.closestBallDistance = ball.distance;
          }
          currentCluster.balls.push_back(&ball2);
          currentCluster.containsOwnBall |=
              ball2.playerNumber == playerConfiguration_->playerNumber;
        }
      }
      // 1. Take the larger cluster.
      if (currentCluster.balls.size() > bestCluster.balls.size() ||
          // This cluster is as large as the best (so it is not smaller).
          (currentCluster.balls.size() == bestCluster.balls.size()
           // 2. The cluster that contains the own ball.
           // There can be multiple clusters which are equally large and contain the own ball
           // (imagine two balls within a distance of 1m). But in that case it does not matter which
           // cluster is chosen because the own ball will be selected out of the cluster anyway.
           && (currentCluster.containsOwnBall
               // 3. The cluster with the smaller robot-ball distance.
               || (!bestCluster.containsOwnBall &&
                   currentCluster.closestBallDistance < bestCluster.closestBallDistance))))
      {
        bestCluster = currentCluster;
      }
    }
    assert(bestCluster.balls.size() <= ballBuffer_.size());
    teamBallModel_->seen = !ballBuffer_.empty();
    teamBallModel_->found = bestCluster.balls.size() > 0.5f * ballBuffer_.size();
    // We can only accept balls that are not seen by ourselfs if we know where we are.
    if (robotPosition_->valid &&
        (teamBallModel_->found || (teamBallModel_->seen && !ballState_->found)))
    {
      float minDistance = std::numeric_limits<float>::max();
      for (auto& bestBall : bestCluster.balls)
      {
        if (bestBall->playerNumber == playerConfiguration_->playerNumber)
        {
          teamBallModel_->position = bestBall->ball.position;
          teamBallModel_->velocity = bestBall->ball.velocity;
          teamBallModel_->ballType = TeamBallModel::BallType::SELF;
          break;
        }
        else if (bestBall->distance < minDistance)
        {
          minDistance = bestBall->distance;
          teamBallModel_->position = bestBall->ball.position;
          teamBallModel_->velocity = bestBall->ball.velocity;
          teamBallModel_->ballType = TeamBallModel::BallType::TEAM;
        }
      }
      assert(teamBallModel_->ballType != TeamBallModel::BallType::NONE);
    }
    else if (ballState_->found)
    {
      teamBallModel_->position = robotPosition_->pose * ballState_->position;
      teamBallModel_->velocity =
          robotPosition_->pose.calculateGlobalOrientation(ballState_->velocity);
      teamBallModel_->ballType = TeamBallModel::BallType::SELF;
    }
  }
  else
  {
    ballBuffer_.clear();
  }
  if ((gameControllerState_->gameState == GameState::SET &&
       teamBallModel_->ballType == TeamBallModel::BallType::NONE) ||
      gameControllerState_->gameState == GameState::READY)
  {
    teamBallModel_->ballType = TeamBallModel::BallType::RULE;
    teamBallModel_->insideField = true;
    teamBallModel_->seen = false;
    teamBallModel_->found = false;
    if (gameControllerState_->gamePhase == GamePhase::PENALTYSHOOT)
    {
      float pmSign = (gameControllerState_->kickingTeam ? 1.f : -1.f);
      teamBallModel_->position = Vector2f(
          (fieldDimensions_->fieldLength * 0.5f - fieldDimensions_->fieldPenaltyMarkerDistance) *
              pmSign,
          0.f);
    }
    else
    {
      teamBallModel_->position = Vector2f::Zero();
    }
    teamBallModel_->velocity = Vector2f::Zero();
  }
  else
  {
    teamBallModel_->insideField =
        (teamBallModel_->ballType == TeamBallModel::BallType::NONE) ||
        fieldDimensions_->isInsideField(teamBallModel_->position, insideFieldTolerance_());
  }
  debug().update(mount_ + ".teamBallModel", *teamBallModel_);
}
