#include <algorithm>
#include <cassert>
#include <chrono>
#include <cmath>
#include <iostream>

#include "Brain/Behavior/PlayingRoleProvider.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/TeamPlayers.hpp"
#include "Framework/Log/Log.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Hysteresis.hpp"

using PRP = PlayingRoleProvider;

PRP::PlayingRoleProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , useTeamRole_(*this, "useTeamRole", [] {})
  , assignBishop_(*this, "assignBishop", [] {})
  , assignBishopWithLessThanFourFieldPlayers_(*this, "assignBishopWithLessThanFourFieldPlayers",
                                              [] {})
  , playerOneCanBecomeStriker_(*this, "playerOneCanBecomeStriker", [] {})
  , playerOneDistanceThreshold_(*this, "playerOneDistanceThreshold", [] {})
  , keeperTimeToReachBallPenalty_(*this, "keeperTimeToReachBallPenalty", [] {})
  , keeperInGoalDistanceThreshold_(*this, "keeperInGoalDistanceThreshold", [] {})
  , strikeOwnBall_(*this, "strikeOwnBall", [] {})
  , allowFastRoleOverride_(*this, "allowFastRoleOverride", [] {})
  , maxFastRoleOverrideDuration_(*this, "maxFastRoleOverrideDuration", [] {})
  , forceRole_(*this, "forceRole", [] {})
  , shortTermBallSearchDuration_(*this, "shortTermBallSearchDuration", [] {})
  , loserDuration_(*this, "loserDuration", [] {})
  , ballState_(*this)
  , fieldDimensions_(*this)
  , playerConfiguration_(*this)
  , robotPosition_(*this)
  , teamPlayers_(*this)
  , teamBallModel_(*this)
  , gameControllerState_(*this)
  , bodyPose_(*this)
  , cycleInfo_(*this)
  , timeToReachBall_(*this)
  , walkGeneratorOutput_(*this)
  , worldState_(*this)
  , playingRoles_(*this)
  , revolting_(false)
{
  lastAssignment_.resize(6, PlayingRole::NONE);
}

// NOLINTNEXTLINE(readability-function-cognitive-complexity)
void PRP::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  if ((gameControllerState_->gameState != GameState::PLAYING &&
       gameControllerState_->gameState != GameState::READY &&
       gameControllerState_->gameState != GameState::SET) ||
      gameControllerState_->penalty != Penalty::NONE ||
      gameControllerState_->gamePhase != GamePhase::NORMAL)
  {
    lastAssignment_.clear();
    return;
  }

  // check whether we are in ball search
  if (teamBallModel_->ballType == TeamBallModel::BallType::NONE)
  {
    ballSearchState_ = BallSearchState::SHORT_TERM;
    if (cycleInfo_->getAbsoluteTimeDifference(teamBallModel_->timeLastUpdated) >
        shortTermBallSearchDuration_())
    {
      ballSearchState_ = BallSearchState::LONG_TERM;
    }
  }
  else
  {
    ballSearchState_ = BallSearchState::NONE;
  }

  // 0. Resize the playingRoles vector to the maximum player number to get map-like access
  unsigned int maxNumber = playerConfiguration_->playerNumber;
  for (const auto& teamPlayer : teamPlayers_->players)
  {
    if (maxNumber < teamPlayer.playerNumber)
    {
      maxNumber = teamPlayer.playerNumber;
    }
  }
  playingRoles_->playerRoles.resize(maxNumber, PlayingRole::NONE);

  // 1. Integrate forced (configured) role.
  if (forceRole_() != "none")
  {
    const PlayingRole newRole = toRole(forceRole_());
    updateRole(playerConfiguration_->playerNumber, newRole);
    lastAssignment_.resize(playerConfiguration_->playerNumber, PlayingRole::NONE);
    lastAssignment_[playerConfiguration_->playerNumber - 1] = newRole;
    // In this case, no roles for other robots are provided.
    return;
  }

  // 2. Assign striker only if we are not in ball search;
  loserAssigned_ = false;
  if (ballSearchState_ == BallSearchState::NONE)
  {
    assignStriker();
  }
  // Assign loser if last striker was closer to the ball than 0.5m and ball search started less
  // than a certain time ago
  else if (ballSearchState_ == BallSearchState::SHORT_TERM &&
           cycleInfo_->getAbsoluteTimeDifference(teamBallModel_->timeLastUpdated) <
               loserDuration_() &&
           lastStrikerNumber_ != 0)
  {
    // assign loser role to player that previously was striker
    updateRole(lastStrikerNumber_, PlayingRole::LOSER);
    loserAssigned_ = true;
  }

  // 3. Assign keeper
  const bool keeperAssigned = assignKeeper();

  // 4. if no keeper was assigned or it is far away, assign a replacement keeper
  if (!keeperAssigned || playerOneIsFarAway())
  {
    assignReplacementKeeper();
  }
  // 5. Assign remaining players to other roles
  assignRemainingPlayerRoles();

  // 6.1 Start revolution if we assigned ourselves as striker (ignore team role for max 5 seconds).
  bool justBecameStriker =
      lastAssignment_[playerConfiguration_->playerNumber - 1] != PlayingRole::STRIKER &&
      playingRoles_->role == PlayingRole::STRIKER;
  bool revolutionJustStarted = cycleInfo_->getAbsoluteTimeDifference(startOfLastRevolution_) <
                               maxFastRoleOverrideDuration_();
  if (allowFastRoleOverride_() && (justBecameStriker || revolutionJustStarted) &&
      playingRoles_->role == PlayingRole::STRIKER)
  {
    if (!revolting_)
    {
      // We just started a new revolution
      revolting_ = true;
      startOfLastRevolution_ = cycleInfo_->startTime;
      Log<M_BRAIN>(LogLevel::FANCY)
          << "Player " << playerConfiguration_->playerNumber << " just started a revolution!";
    }
  }
  // 6.2 Overwrite the own role if an eligible role provider has one for us.
  else if (useTeamRole_() || gameControllerState_->gameState != GameState::PLAYING)
  {
    if (revolting_)
    {
      revolting_ = false;
      Log<M_BRAIN>(LogLevel::FANCY)
          << "Player " << playerConfiguration_->playerNumber << " stopped revolting :)";
    }

    unsigned int minPlayerNumber = playerConfiguration_->playerNumber;
    for (const auto& teamPlayer : teamPlayers_->players)
    {
      if (!teamPlayer.penalized && teamPlayer.playerNumber < minPlayerNumber &&
          teamPlayer.roleAssignments.size() >= playerConfiguration_->playerNumber &&
          teamPlayer.roleAssignments[playerConfiguration_->playerNumber - 1] != PlayingRole::NONE)
      {
        minPlayerNumber = teamPlayer.playerNumber;
        playingRoles_->role = teamPlayer.roleAssignments[playerConfiguration_->playerNumber - 1];
      }
    }
  }

  // 7. strike the own ball when there is no majority found in TeamBallFilter but an own ball is
  // confident
  if (strikeOwnBall_() && !teamBallModel_->found && ballState_->confident)
  {
    const Vector2f absBallPosition = robotPosition_->robotToField(ballState_->position);
    const Vector2f target(fieldDimensions_->fieldLength / 2.f, 0.f);
    const auto ownTimeToReachBall = timeToReachBall_->estimateTimeToReachBall(
        robotPosition_->pose, absBallPosition, target, bodyPose_->fallen, true,
        walkGeneratorOutput_->maxVelocityComponents);
    bool smallestTimeToReachBall = true;
    for (const auto& teamPlayer : teamPlayers_->players)
    {
      // This is a hack. We use our own maxVelocity as it is hard coded to the same value for all
      // robots anyway.
      const auto teamPlayerTimeToReachBall = timeToReachBall_->estimateTimeToReachBall(
          teamPlayer.pose, absBallPosition, target, teamPlayer.fallen, true,
          walkGeneratorOutput_->maxVelocityComponents);
      if (teamPlayerTimeToReachBall < ownTimeToReachBall)
      {
        smallestTimeToReachBall = false;
        break;
      }
    }
    if (smallestTimeToReachBall)
    {
      playingRoles_->role = PlayingRole::STRIKER;
    }
  }

  // 8. Set last assignment (for hysteresis).
  lastAssignment_ = playingRoles_->playerRoles;
}

void PRP::assignStriker()
{
  auto smallestTimeToReachBall =
      timeToReachBall_->valid
          ? actualTimeToReachBall(playerConfiguration_->playerNumber,
                                  std::chrono::duration_cast<std::chrono::milliseconds>(
                                      timeToReachBall_->timeToReachBall),
                                  timeToReachBall_->timeToReachBallStriker)
          : Clock::duration::max();
  unsigned int strikerNumber = (timeToReachBall_->valid && playingRoles_->role == PlayingRole::NONE)
                                   ? playerConfiguration_->playerNumber
                                   : 0;
  if (!playerOneCanBecomeStriker_() && playerConfiguration_->playerNumber == 1)
  {
    smallestTimeToReachBall = Clock::duration::max();
    strikerNumber = 0;
  }
  for (const auto& teamPlayer : teamPlayers_->players)
  {
    if (teamPlayer.penalized ||
        playingRoles_->playerRoles[teamPlayer.playerNumber - 1] != PlayingRole::NONE)
    {
      continue;
    }
    if (!playerOneCanBecomeStriker_() && teamPlayer.playerNumber == 1)
    {
      continue;
    }
    const auto ttrb = actualTimeToReachBall(
        teamPlayer.playerNumber,
        cycleInfo_->getAbsoluteTimeDifference(teamPlayer.timeWhenReachBall),
        cycleInfo_->getAbsoluteTimeDifference(teamPlayer.timeWhenReachBallStriker));
    if (ttrb < smallestTimeToReachBall)
    {
      strikerNumber = teamPlayer.playerNumber;
      smallestTimeToReachBall = ttrb;
    }
  }
  if (strikerNumber != 0)
  {
    updateRole(strikerNumber, PlayingRole::STRIKER);
    // remember the striker number
    lastStrikerNumber_ = strikerNumber;
  }
}

bool PRP::assignKeeper()
{
  // the keeper is only assigned to a robot with player number one
  if (playingRoles_->role == PlayingRole::NONE && playerConfiguration_->playerNumber == 1)
  {
    updateRole(1, PlayingRole::KEEPER);
    return true;
  }
  for (const auto& teamPlayer : teamPlayers_->players)
  {
    if (teamPlayer.penalized ||
        playingRoles_->playerRoles[teamPlayer.playerNumber - 1] != PlayingRole::NONE)
    {
      continue;
    }
    if (teamPlayer.playerNumber == 1)
    {
      updateRole(1, PlayingRole::KEEPER);
      return true;
    }
  }
  return false;
}

void PRP::assignReplacementKeeper()
{
  float smallestDistanceToOwnGoal =
      robotPosition_->valid && (playingRoles_->role == PlayingRole::NONE ||
                                playingRoles_->role == PlayingRole::KEEPER)
          ? getDistanceToGoal(robotPosition_->pose.position(), playerConfiguration_->playerNumber)
          : std::numeric_limits<float>::max();
  unsigned int replaceKeeperNumber =
      (robotPosition_->valid && playingRoles_->role == PlayingRole::NONE)
          ? playerConfiguration_->playerNumber
          : 0;
  for (const auto& teamPlayer : teamPlayers_->players)
  {
    // Skip all players that are penalized or already have a role assigned (except keeper)
    if (teamPlayer.penalized ||
        (playingRoles_->playerRoles[teamPlayer.playerNumber - 1] != PlayingRole::NONE &&
         playingRoles_->playerRoles[teamPlayer.playerNumber - 1] != PlayingRole::KEEPER))
    {
      continue;
    }
    const float distanceToOwnGoal =
        getDistanceToGoal(teamPlayer.pose.position(), teamPlayer.playerNumber);
    if (distanceToOwnGoal < smallestDistanceToOwnGoal)
    {
      replaceKeeperNumber = teamPlayer.playerNumber;
      smallestDistanceToOwnGoal = distanceToOwnGoal;
    }
  }
  // only assign replacement keeper if we have a valid candidate.
  // Note: if replaceKeeperNumber == 1 then the keeper is the nearest player to the goal. No
  // replacement keeper is assigned then
  if (replaceKeeperNumber != 0 && replaceKeeperNumber != 1)
  {
    updateRole(replaceKeeperNumber, PlayingRole::REPLACEMENT_KEEPER);
  }
}

float PRP::getDistanceToGoal(const Vector2f& position, const unsigned int playerNumber) const
{
  float distanceToOwnGoal = (position - Vector2f(-fieldDimensions_->fieldLength / 2, 0.f)).norm();
  if (lastRoleOf(playerNumber) == PlayingRole::KEEPER)
  {
    distanceToOwnGoal -= 0.5f;
  }
  return distanceToOwnGoal;
}

bool PRP::playerOneIsFarAway()
{
  const Vector2f absOwnGoalPosition(-fieldDimensions_->fieldLength / 2, 0.f);
  Vector2f playerOneToOwnGoal =
      Vector2f(std::numeric_limits<float>::max(), std::numeric_limits<float>::max());
  if (playerConfiguration_->playerNumber == 1)
  {
    playerOneToOwnGoal = absOwnGoalPosition - robotPosition_->pose.position();
  }
  else
  {
    for (const auto& teamPlayer : teamPlayers_->players)
    {
      if (teamPlayer.penalized)
      {
        continue;
      }
      if (teamPlayer.playerNumber == 1)
      {
        playerOneToOwnGoal = absOwnGoalPosition - teamPlayer.pose.position();
        break;
      }
    }
  }
  const float hysteresis = 0.25f;
  playerOneWasFarAway_ = Hysteresis::greaterThan(
      playerOneToOwnGoal.norm(), playerOneDistanceThreshold_(), hysteresis, playerOneWasFarAway_);
  return playerOneWasFarAway_;
}

void PRP::assignRemainingPlayerRoles()
{
  std::vector<Player> remainingPlayers;
  if (playingRoles_->playerRoles[playerConfiguration_->playerNumber - 1] == PlayingRole::NONE)
  {
    remainingPlayers.emplace_back(playerConfiguration_->playerNumber,
                                  robotPosition_->pose.position());
  }
  for (const auto& teamPlayer : teamPlayers_->players)
  {
    if (teamPlayer.penalized ||
        playingRoles_->playerRoles[teamPlayer.playerNumber - 1] != PlayingRole::NONE)
    {
      continue;
    }
    remainingPlayers.emplace_back(teamPlayer.playerNumber, teamPlayer.pose.position());
  }
  // With no or one remaining robot no hysteresis or fancy selection needs to be done.
  if (remainingPlayers.empty())
  {
    return;
  }
  // When in long term ball search, all remaining players will be searchers
  if (ballSearchState_ == BallSearchState::LONG_TERM)
  {
    for (const auto& player : remainingPlayers)
    {
      updateRole(player.playerNumber, PlayingRole::SEARCHER);
    }
    return;
  }
  // The x coordinates are artificially increased/decreased depending on the last role.
  // This ensures decision stability.
  for (auto& player : remainingPlayers)
  {
    switch (lastRoleOf(player.playerNumber))
    {
      case PlayingRole::DEFENDER:
        player.position.x() -= 0.2f;
        break;
      case PlayingRole::SUPPORT_STRIKER:
        player.position.x() += 0.2f;
        break;
      case PlayingRole::BISHOP:
        player.position.x() += 0.3f;
        break;
      default:
        break;
    }
  }
  // sort all remaining players
  std::sort(remainingPlayers.begin(), remainingPlayers.end(),
            [](const Player& p1, const Player& p2) { return p1.position.x() < p2.position.x(); });
  // When in short term ball search, make one robot defender and the remaining ones searcher
  if (ballSearchState_ == BallSearchState::SHORT_TERM)
  {
    // When there is no loser, the first remaining player should be searcher
    if (!loserAssigned_)
    {
      updateRole(remainingPlayers[remainingPlayers.size() - 1].playerNumber, PlayingRole::SEARCHER);
      remainingPlayers.pop_back();
      // Check again for emptiness
      if (remainingPlayers.empty())
      {
        return;
      }
    }
    updateRole(remainingPlayers[0].playerNumber, PlayingRole::DEFENDER);
    remainingPlayers.erase(remainingPlayers.begin());
    // If players remain, make them searcher
    for (auto& player : remainingPlayers)
    {
      updateRole(player.playerNumber, PlayingRole::SEARCHER);
    }
    return;
  }
  // We are not in ball search
  if (remainingPlayers.size() == 1)
  {
    // One remaining field player should be defender.
    updateRole(remainingPlayers[0].playerNumber, PlayingRole::DEFENDER);
    return;
  }
  auto bishopOrSupporter = [&](unsigned int candidate) -> PlayingRole {
    // During free kicks, we want to have a bishop as a pass target even with 4 players
    if ((gameControllerState_->setPlay == SetPlay::KICK_IN ||
         gameControllerState_->setPlay == SetPlay::GOAL_KICK ||
         gameControllerState_->setPlay == SetPlay::CORNER_KICK ||
         gameControllerState_->setPlay == SetPlay::PUSHING_FREE_KICK) &&
        gameControllerState_->kickingTeam)
    {
      return PlayingRole::BISHOP;
    }
    if (!assignBishop_())
    {
      return PlayingRole::SUPPORT_STRIKER;
    }
    if (remainingPlayers.size() < 3 && !assignBishopWithLessThanFourFieldPlayers_())
    {
      return PlayingRole::SUPPORT_STRIKER;
    }
    // If the ball is far from the own goal, a bishop is useful because the two defenders can
    // take the supporting role of catching lost striker balls and the bishop can take balls that go
    // into the opponent's half. On the other hand, when the ball is near the opponent's goal, no
    // bishop is needed anymore and the defenders are far from the ball so there should be a
    // supporter that collects balls that are lost by the striker.
    bool hadBishop = false;
    for (const auto& player : remainingPlayers)
    {
      // We only want the hadBishop bonus if the same robot would become bishop again.
      if (lastRoleOf(player.playerNumber) == PlayingRole::BISHOP &&
          player.playerNumber == candidate)
      {
        hadBishop = true;
        break;
      }
    }

    if (gameControllerState_->setPlay != SetPlay::NONE)
    {
      // We want a bishop if we are the kicking team. Also, a bishop is assigned if we had one
      // before to prevent it from crossing the field when we are not the kicking team
      if (gameControllerState_->kickingTeam || hadBishop)
      {
        return PlayingRole::BISHOP;
      }
      else
      {
        return PlayingRole::SUPPORT_STRIKER;
      }
    }

    bool assignBishop = hadBishop;
    if (teamBallModel_->ballType != TeamBallModel::BallType::NONE)
    {
      const float threshAssignBishop = hadBishop ? 1.0f : 0.0f;
      assignBishop = (teamBallModel_->absPosition.x() < threshAssignBishop);
    }
    return (assignBishop ? PlayingRole::BISHOP : PlayingRole::SUPPORT_STRIKER);
  };
  if (remainingPlayers.size() == 2)
  {
    // Of two remaining field players one should be defender and the other one should be supporter
    // or bishop.
    updateRole(remainingPlayers[0].playerNumber, PlayingRole::DEFENDER);
    updateRole(remainingPlayers[1].playerNumber,
               bishopOrSupporter(remainingPlayers[1].playerNumber));
  }
  else if (remainingPlayers.size() == 3)
  {
    // This is the maximum situation in normal games
    // One robot should be defender, one should be supporter, and one should be bishop.
    updateRole(remainingPlayers[0].playerNumber, PlayingRole::DEFENDER);
    updateRole(remainingPlayers[1].playerNumber, PlayingRole::SUPPORT_STRIKER);
    updateRole(remainingPlayers[2].playerNumber, PlayingRole::BISHOP);
  }
  else
  {
    Log<M_BRAIN>(LogLevel::ERROR)
        << "Too many remaining players. There should never be more than 5 players.";
    assert(false);
  }
}


void PRP::updateRole(const unsigned int playerNumber, const PlayingRole role)
{
  playingRoles_->playerRoles[playerNumber - 1] = role;
  if (playerNumber == playerConfiguration_->playerNumber)
  {
    playingRoles_->role = role;
  }
}

PlayingRole PlayingRoleProvider::toRole(const std::string& configRole) const
{
  if (configRole == "keeper")
  {
    return PlayingRole::KEEPER;
  }
  if (configRole == "defender")
  {
    return PlayingRole::DEFENDER;
  }
  if (configRole == "striker")
  {
    return PlayingRole::STRIKER;
  }
  if (configRole == "supportStriker")
  {
    return PlayingRole::SUPPORT_STRIKER;
  }
  if (configRole == "bishop")
  {
    return PlayingRole::BISHOP;
  }
  if (configRole == "replacementKeeper")
  {
    return PlayingRole::REPLACEMENT_KEEPER;
  }
  if (configRole == "loser")
  {
    return PlayingRole::LOSER;
  }
  if (configRole == "searcher")
  {
    return PlayingRole::SEARCHER;
  }
  throw std::runtime_error("Unknown PlayingRole '" + configRole + "'");
}

Clock::duration PRP::actualTimeToReachBall(const unsigned int playerNumber,
                                           const Clock::duration& timeToReachBall,
                                           const Clock::duration& timeToReachBallStriker)
{
  if (playingRoles_->playerRoles[playerNumber - 1] != PlayingRole::NONE)
  {
    return Clock::duration::max();
  }
  const bool wasStriker = lastRoleOf(playerNumber) == PlayingRole::STRIKER;
  if (wasStriker)
  {
    // last striker has bonus
    return timeToReachBallStriker;
  }
  const bool wasKeeper = lastRoleOf(playerNumber) == PlayingRole::KEEPER;
  const bool wasReplacementKeeper = lastRoleOf(playerNumber) == PlayingRole::REPLACEMENT_KEEPER;
  const float robotToGoal =
      (robotPosition_->pose.position() - Vector2f{-fieldDimensions_->fieldLength / 2, 0.f}).norm();
  const float hysteresis = 0.25f;
  inGoal_ =
      Hysteresis::smallerThan(robotToGoal, keeperInGoalDistanceThreshold_(), hysteresis, inGoal_);
  if ((wasKeeper || wasReplacementKeeper) && inGoal_)
  {
    // last keeper and replacement keeper get penalty if they were in the goal
    return timeToReachBall + keeperTimeToReachBallPenalty_();
  }
  return timeToReachBall;
}

PlayingRole PRP::lastRoleOf(const unsigned int playerNumber) const
{
  return playerNumber <= lastAssignment_.size() ? lastAssignment_[playerNumber - 1]
                                                : PlayingRole::NONE;
}
