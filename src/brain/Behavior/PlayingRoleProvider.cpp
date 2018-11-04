#include <algorithm>
#include <cassert>
#include <cmath>

#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Hysteresis.hpp"

#include "PlayingRoleProvider.hpp"


using PRP = PlayingRoleProvider;

PRP::PlayingRoleProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , useTeamRole_(*this, "useTeamRole", [] {})
  , assignBishop_(*this, "assignBishop", [] {})
  , assignBishopWithLessThanFourFieldPlayers_(*this, "assignBishopWithLessThanFourFieldPlayers",
                                              [] {})
  , playerOneCanBecomeStriker_(*this, "playerOneCanBecomeStriker", [] {})
  , allowReplacementKeeper_(*this, "allowReplacementKeeper", [] {})
  , playerOneDistanceThreshold_(*this, "playerOneDistanceThreshold", [] {})
  , keeperTimeToReachBallPenalty_(*this, "keeperTimeToReachBallPenalty", [] {})
  , forceRole_(*this, "forceRole", [] {})
  , fieldDimensions_(*this)
  , playerConfiguration_(*this)
  , robotPosition_(*this)
  , teamPlayers_(*this)
  , teamBallModel_(*this)
  , gameControllerState_(*this)
  , bodyPose_(*this)
  , cycleInfo_(*this)
  , timeToReachBall_(*this)
  , worldState_(*this)
  , playingRoles_(*this)
{
}

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

  // 2. Find striker if there is no ongoing enemy free kick

  // check for referee mistake: If the ball is inside our half AND we are NOT the kickingTeam
  // AND there is an ongoing GOALFreeKick: The ref most likely clicked the wrong GoalFreeKick
  // button. We will ignore the GC in this case.
  const bool forceFreeKickStriker =
      teamBallModel_->seen && gameControllerState_->setPlay == SetPlay::GOAL_FREE_KICK &&
      !gameControllerState_->kickingTeam && worldState_->ballInOwnHalf;

  if (gameControllerState_->setPlay == SetPlay::NONE || forceFreeKickStriker ||
      gameControllerState_->kickingTeam)
  {
    assignStriker();
  }

  // 3. Assign keeper
  assignKeeper();

  // 4. if player one is far away (or nonexistent) assign a replacement keeper
  if (playerOneIsFarAway() && allowReplacementKeeper_())
  {
    assignReplacementKeeper();
  }
  // 5. Assign remaining players to other roles
  assignRemainingPlayerRoles();

  // 6. Overwrite the own role if an eligible role provider has one for us.
  if (useTeamRole_() || gameControllerState_->gameState != GameState::PLAYING)
  {
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

  // 7. Set last assignment (for hysteresis).
  lastAssignment_ = playingRoles_->playerRoles;
}

void PRP::assignStriker()
{
  float smallestTimeToReachBall =
      timeToReachBall_->valid ? actualTimeToReachBall(playerConfiguration_->playerNumber,
                                                      timeToReachBall_->timeToReachBall,
                                                      timeToReachBall_->timeToReachBallStriker)
                              : std::numeric_limits<float>::max();
  unsigned int strikerNumber =
      (timeToReachBall_->valid && playingRoles_->role == PlayingRole::NONE)
          ? playerConfiguration_->playerNumber
          : 0;
  if (!playerOneCanBecomeStriker_() && playerConfiguration_->playerNumber == 1)
  {
    smallestTimeToReachBall = std::numeric_limits<float>::max();
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
    const float ttrb = actualTimeToReachBall(
        teamPlayer.playerNumber, cycleInfo_->getTimeDiff(teamPlayer.timeWhenReachBall),
        cycleInfo_->getTimeDiff(teamPlayer.timeWhenReachBallStriker));
    if (ttrb < smallestTimeToReachBall)
    {
      strikerNumber = teamPlayer.playerNumber;
      smallestTimeToReachBall = ttrb;
    }
  }
  if (strikerNumber != 0)
  {
    updateRole(strikerNumber, PlayingRole::STRIKER);
  }
}

void PRP::assignKeeper()
{
  // the keeper is only assigned to a robot with player number one
  if (playingRoles_->role == PlayingRole::NONE && playerConfiguration_->playerNumber == 1)
  {
    updateRole(1, PlayingRole::KEEPER);
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
    }
  }
}

void PRP::assignReplacementKeeper()
{
  float smallestDistanceToOwnGoal =
      robotPosition_->valid && playingRoles_->role == PlayingRole::NONE
          ? getDistanceToGoal(robotPosition_->pose.position, playerConfiguration_->playerNumber)
          : std::numeric_limits<float>::max();
  unsigned int replaceKeeperNumber =
      (robotPosition_->valid && playingRoles_->role == PlayingRole::NONE)
          ? playerConfiguration_->playerNumber
          : 0;
  for (const auto& teamPlayer : teamPlayers_->players)
  {
    if (teamPlayer.penalized ||
        playingRoles_->playerRoles[teamPlayer.playerNumber - 1] != PlayingRole::NONE)
    {
      continue;
    }
    const float distanceToOwnGoal =
        getDistanceToGoal(teamPlayer.pose.position, teamPlayer.playerNumber);
    if (distanceToOwnGoal < smallestDistanceToOwnGoal)
    {
      replaceKeeperNumber = teamPlayer.playerNumber;
      smallestDistanceToOwnGoal = distanceToOwnGoal;
    }
  }
  if (replaceKeeperNumber != 0)
  {
    updateRole(replaceKeeperNumber, PlayingRole::REPLACEMENT_KEEPER);
  }
}

float PRP::getDistanceToGoal(const Vector2f position, const unsigned int playerNumber) const
{
  float distanceToOwnGoal = (position - Vector2f(-fieldDimensions_->fieldLength / 2, 0.f)).norm();
  if (lastRoleOf(playerNumber) == PlayingRole::KEEPER)
  {
    distanceToOwnGoal -= 0.2f;
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
    playerOneToOwnGoal = absOwnGoalPosition - robotPosition_->pose.position;
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
        playerOneToOwnGoal = absOwnGoalPosition - teamPlayer.pose.position;
        break;
      }
    }
  }
  const float hysteresis = 0.25f;
  playerOneWasFarAway_ = Hysteresis<float>::greaterThan(playerOneToOwnGoal.norm(), playerOneDistanceThreshold_(), hysteresis, playerOneWasFarAway_);
  return playerOneWasFarAway_;
}

void PRP::assignRemainingPlayerRoles()
{
  std::vector<Player> remainingPlayers;
  if (playingRoles_->playerRoles[playerConfiguration_->playerNumber - 1] == PlayingRole::NONE)
  {
    remainingPlayers.emplace_back(playerConfiguration_->playerNumber,
                                  robotPosition_->pose.position.x());
  }
  for (const auto& teamPlayer : teamPlayers_->players)
  {
    if (teamPlayer.penalized ||
        playingRoles_->playerRoles[teamPlayer.playerNumber - 1] != PlayingRole::NONE)
    {
      continue;
    }
    remainingPlayers.emplace_back(teamPlayer.playerNumber, teamPlayer.pose.position.x());
  }
  // With no or one remaining robot no hysteresis or fancy selection needs to be done.
  if (remainingPlayers.empty())
  {
    return;
  }
  else if (remainingPlayers.size() == 1)
  {
    // One remaining field player should be defender.
    updateRole(remainingPlayers[0].playerNumber, PlayingRole::DEFENDER);
    return;
  }
  // The x coordinates are artificially increased/decreased depending on the last role.
  // This ensures decision stability.
  for (auto& player : remainingPlayers)
  {
    switch (lastRoleOf(player.playerNumber))
    {
      case PlayingRole::DEFENDER:
        player.x -= 0.2f;
        break;
      case PlayingRole::SUPPORT_STRIKER:
        player.x += 0.2f;
        break;
      case PlayingRole::BISHOP:
        player.x += 0.3f;
        break;
      default:
        break;
    }
  }
  std::sort(remainingPlayers.begin(), remainingPlayers.end(),
            [](const Player& p1, const Player& p2) { return p1.x < p2.x; });
  auto bishopOrSupporter = [&](unsigned int candidate) -> PlayingRole {
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

    bool assignBishop;
    if (teamBallModel_->ballType != TeamBallModel::BallType::NONE)
    {
      const float threshAssignBishop = hadBishop ? 2.f : 1.f;
      assignBishop = (teamBallModel_->position.x() < threshAssignBishop);
    }
    else
    {
      assignBishop = hadBishop;
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
    // This is the maximum situation in normal games.
    // Two robots should be defender and one should be supporter or bishop.
    updateRole(remainingPlayers[0].playerNumber, PlayingRole::DEFENDER);
    updateRole(remainingPlayers[1].playerNumber, PlayingRole::DEFENDER);
    updateRole(remainingPlayers[2].playerNumber,
               bishopOrSupporter(remainingPlayers[2].playerNumber));
  }
  else if (remainingPlayers.size() == 4)
  {
    // This happens only in mixed team games.
    // Full line-up, two defenders, one supporter and one bishop.
    updateRole(remainingPlayers[0].playerNumber, PlayingRole::DEFENDER);
    updateRole(remainingPlayers[1].playerNumber, PlayingRole::DEFENDER);
    updateRole(remainingPlayers[2].playerNumber, PlayingRole::SUPPORT_STRIKER);
    updateRole(remainingPlayers[3].playerNumber, PlayingRole::BISHOP);
  }
  else
  {
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
  else if (configRole == "defender")
  {
    return PlayingRole::DEFENDER;
  }
  else if (configRole == "striker")
  {
    return PlayingRole::STRIKER;
  }
  else if (configRole == "supportStriker")
  {
    return PlayingRole::SUPPORT_STRIKER;
  }
  else if (configRole == "bishop")
  {
    return PlayingRole::BISHOP;
  }
  else if (configRole == "replacementKeeper")
  {
    return PlayingRole::REPLACEMENT_KEEPER;
  }
  throw std::runtime_error("Unknown PlayingRole '" + configRole + "'!");
}

float PRP::actualTimeToReachBall(const unsigned int playerNumber, const float timeToReachBall,
                                 const float timeToReachBallStriker) const
{
  if (playingRoles_->playerRoles[playerNumber - 1] != PlayingRole::NONE)
  {
    return std::numeric_limits<float>::max();
  }
  const bool wasStriker = lastRoleOf(playerNumber) == PlayingRole::STRIKER;
  if (wasStriker)
  {
    // last striker has bonus
    return timeToReachBallStriker;
  }
  const bool wasKeeper = lastRoleOf(playerNumber) == PlayingRole::KEEPER;
  const bool wasReplacementKeeper = lastRoleOf(playerNumber) == PlayingRole::REPLACEMENT_KEEPER;
  if(wasKeeper || wasReplacementKeeper)
  {
    // last keeper and replacement keeper get penalty
    return timeToReachBall + keeperTimeToReachBallPenalty_();
  }
  return timeToReachBall;
}

PlayingRole PRP::lastRoleOf(const unsigned int playerNumber) const
{
  return playerNumber <= lastAssignment_.size() ? lastAssignment_[playerNumber - 1]
                                                : PlayingRole::NONE;
}
