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
      // reset number of measurements in case it "jumped" too far.
      if ((ball.ball.position - (pose * relBallPosition)).norm() > maxCompatibilityDistance_())
      {
        ball.timeFirstSeen = timestamp;
      }
      ball.ball.position = pose * relBallPosition;
      ball.ball.velocity = pose.calculateGlobalOrientation(relBallVelocity);
      ball.distance = relBallPosition.norm();
      ball.timeLastSeen = timestamp;
      return;
    }
  }
  TeamPlayerBall ball;
  ball.playerNumber = playerNumber;
  ball.timeFirstSeen = timestamp;
  ball.ball.position = pose * relBallPosition;
  ball.ball.velocity = pose.calculateGlobalOrientation(relBallVelocity);
  ball.distance = relBallPosition.norm();
  ball.timeLastSeen = timestamp;
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
                                     return cycleInfo_->getTimeDiff(ball.timeLastSeen) >
                                            minRemoveAge_();
                                   }),
                    ballBuffer_.end());
}

void TeamBallFilter::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  // Reset the filter when we are penalized or in initial or finished.
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
    // Copy and update all balls of our teammates into the local ballBuffer_
    updateBallBuffer();
    // The cluster to be created from the given ball.
    BallCluster clusterUnderConstruction;
    // A collection of all clusters that were created. Contains no duplicates.
    std::vector<BallCluster> allBallClusters;
    // Now create all clusters of all balls
    for (auto& clusterCenter : ballBuffer_)
    {
      // initialize the cluster with the given ball. This will be the cluster's center.
      clusterUnderConstruction.balls.resize(1);
      clusterUnderConstruction.balls[0] = &clusterCenter;
      clusterUnderConstruction.containsOwnBall =
          clusterCenter.playerNumber == playerConfiguration_->playerNumber;
      clusterUnderConstruction.closestBallDistance = clusterCenter.distance;
      clusterUnderConstruction.timeFirstSeen = clusterCenter.timeFirstSeen;
      // find other balls that are near the cluster center
      for (auto& candidate : ballBuffer_)
      {
        // skip the ball that already is included in the cluster.
        if (clusterCenter.playerNumber == candidate.playerNumber)
        {
          continue;
        }
        // only add balls that are near the cluster's center.
        if ((clusterCenter.ball.position - candidate.ball.position).norm() <
            maxCompatibilityDistance_())
        {
          // update the closest ball distance
          if (candidate.distance < clusterUnderConstruction.closestBallDistance)
          {
            clusterUnderConstruction.closestBallDistance = candidate.distance;
          }
          // update the firstSeen timestamp.
          if (candidate.timeFirstSeen < clusterCenter.timeFirstSeen)
          {
            clusterUnderConstruction.timeFirstSeen = candidate.timeFirstSeen;
          }
          // add the ball to the cluster.
          clusterUnderConstruction.balls.push_back(&candidate);
          clusterUnderConstruction.containsOwnBall |=
              candidate.playerNumber == playerConfiguration_->playerNumber;
        }
      } // Finished generating the clusterUnderConstruction

      // sort the clusterUnderConstruction according to the playerNumbers that reported the balls
      std::sort(clusterUnderConstruction.balls.begin(), clusterUnderConstruction.balls.end(),
                [](auto& a, auto& b) { return a->playerNumber < b->playerNumber; });

      // assume that allBallClusters do not contain the clusterUnderConstruction
      bool isDuplicate = false;
      // check allBallClusters for duplicates (eg. cluster(3,4) == cluster(4,3))
      for (auto& potentialDuplicate : allBallClusters)
      {
        if (potentialDuplicate.balls.size() == clusterUnderConstruction.balls.size())
        {
          // assume that the clusterUnderConstruction is a duplicate of potentialDuplicate
          isDuplicate = true;
          // compare the numbers of the player that have seen the given balls. Whenever the number
          // mismatches for the same index the cluster is not a duplicate (as the clusters are
          // sorted accordingly)
          for (uint8_t index = 0; index < clusterUnderConstruction.balls.size(); index++)
          {
            if (clusterUnderConstruction.balls[index]->playerNumber !=
                potentialDuplicate.balls[index]->playerNumber)
            {
              isDuplicate = false;
              break;
            }
          }
        }
      } // Finished checking for duplicates
      // add the clusterUnderConstruction to allBallClusters when allClusters is empty and it is not
      // a duplicate
      if (!isDuplicate || allBallClusters.empty())
      {
        allBallClusters.push_back(clusterUnderConstruction);
      }
    } // Finished generating clusters

    // Find the best cluster that will be returned as the teamBall.
    if (!allBallClusters.empty())
    {
      // Sort allBallClusters so that allBallClusters[0] contains the best* cluster.
      // * see lambda function below
      std::sort(allBallClusters.begin(), allBallClusters.end(),
                [](const BallCluster& a, const BallCluster& b) {
                  bool clusterAIsBetter = false;
                  // 1. Take the larger cluster
                  // (Trust a cluster that contains more balls from different robots)
                  if (a.balls.size() > b.balls.size())
                  {
                    clusterAIsBetter = true;
                  }
                  // This cluster is as large as the best (so it is not smaller).
                  else if (a.balls.size() == b.balls.size() && a.timeFirstSeen < b.timeFirstSeen)
                  {
                    clusterAIsBetter = true;
                  }

                  return clusterAIsBetter;
                });

      allBallClusters[0].isBestCluster = true; // for debug purposes only
      BallCluster& bestCluster = allBallClusters[0];

      debug().update(mount_ + ".allBallClusters", allBallClusters);

      // The bestCluster must not contain more balls than detected.
      assert(bestCluster.balls.size() <= ballBuffer_.size());
      teamBallModel_->seen = true; // ball buffer was not empty
      teamBallModel_->found = bestCluster.balls.size() > 0.5f * ballBuffer_.size();
      // We can only accept balls that are not seen by ourselves if we know where we are.
      // Otherwise the relative position will not be correct. In this case we will return the
      // own ball if available.
      if (robotPosition_->valid &&
          (teamBallModel_->found || (teamBallModel_->seen && !ballState_->found)))
      {
        // choose the ball to return from the best cluster.
        float minDistance = std::numeric_limits<float>::max();
        for (auto& bestBall : bestCluster.balls)
        {
          // the own ball is always preferred as the relative position does not contain the
          // self localization offset of other players.
          if (bestBall->playerNumber == playerConfiguration_->playerNumber)
          {
            teamBallModel_->position = bestBall->ball.position;
            teamBallModel_->velocity = bestBall->ball.velocity;
            teamBallModel_->ballType = TeamBallModel::BallType::SELF;
            teamBallModel_->timeLastUpdated = bestBall->timeLastSeen;
            break;
          }
          // Take the ball that is closest to the detecting robot.
          else if (bestBall->distance < minDistance)
          {
            minDistance = bestBall->distance;
            teamBallModel_->position = bestBall->ball.position;
            teamBallModel_->velocity = bestBall->ball.velocity;
            teamBallModel_->ballType = TeamBallModel::BallType::TEAM;
            teamBallModel_->timeLastUpdated = bestBall->timeLastSeen;
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
        teamBallModel_->timeLastUpdated = ballState_->timeWhenLastSeen;
      }
    }
  }
  else
  {
    // We are in ready state
    ballBuffer_.clear();
  }
  // During SET and READY the rules can help us to find the ball. When no ball was selected in the
  // process above, the teamBall will be set to the position known from the rules.
  if ((gameControllerState_->gameState == GameState::SET &&
       teamBallModel_->ballType == TeamBallModel::BallType::NONE) ||
      gameControllerState_->gameState == GameState::READY)
  {
    teamBallModel_->ballType = TeamBallModel::BallType::RULE;
    teamBallModel_->insideField = true;
    teamBallModel_->seen = false;
    teamBallModel_->found = false;
    teamBallModel_->timeLastUpdated = cycleInfo_->startTime;
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
