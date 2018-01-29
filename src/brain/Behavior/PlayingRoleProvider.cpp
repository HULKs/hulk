#include <algorithm>
#include <cassert>
#include <cmath>

#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"

#include "PlayingRoleProvider.hpp"


using PRP = PlayingRoleProvider;

PRP::PlayingRoleProvider(const ModuleManagerInterface& manager)
  : Module(manager, "PlayingRoleProvider")
  , useTeamRole_(*this, "useTeamRole", [] {})
  , forceRole_(*this, "forceRole", [] {})
  , playerConfiguration_(*this)
  , robotPosition_(*this)
  , teamPlayers_(*this)
  , teamBallModel_(*this)
  , gameControllerState_(*this)
  , bodyPose_(*this)
  , strikerAction_(*this)
  , cycleInfo_(*this)
  , timeToReachBall_(*this)
  , playingRoles_(*this)
{
}

void PRP::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  if ((gameControllerState_->state != GameState::PLAYING && gameControllerState_->state != GameState::READY && gameControllerState_->state != GameState::SET) ||
      gameControllerState_->penalty != Penalty::NONE || gameControllerState_->secondary != SecondaryState::NORMAL)
  {
    lastAssignment_.clear();
    return;
  }

  // 0. Resize the playingRoles vector to the maximum player number to get map-like access
  unsigned int maxNumber = playerConfiguration_->playerNumber;
  for (auto& teamPlayer : teamPlayers_->players)
  {
    if (maxNumber < teamPlayer.playerNumber)
    {
      maxNumber = teamPlayer.playerNumber;
    }
  }
  playingRoles_->playerRoles.resize(maxNumber, PlayingRole::NONE);

  // 1. Let number 1 be the keeper (overrides even forceRole_)
  updateRole(1, PlayingRole::KEEPER);


  // 2. Integrate forced (configured) role.
  if (forceRole_() != "none")
  {
    const PlayingRole newRole = toRole(forceRole_());
    updateRole(playerConfiguration_->playerNumber, newRole);
    lastAssignment_.resize(playerConfiguration_->playerNumber, PlayingRole::NONE);
    lastAssignment_[playerConfiguration_->playerNumber - 1] = newRole;
    // In this case, no roles for other robots are provided.
    return;
  }

  // 3. Find striker.
  float smallestTimeToReachBall = timeToReachBall_->valid ? actualTimeToReachBall(playerConfiguration_->playerNumber, timeToReachBall_->timeToReachBall,
                                                                                  timeToReachBall_->timeToReachBallStriker)
                                                          : std::numeric_limits<float>::max();
  unsigned int strikerNumber = (timeToReachBall_->valid && playingRoles_->role == PlayingRole::NONE) ? playerConfiguration_->playerNumber : 0;
  for (auto& teamPlayer : teamPlayers_->players)
  {
    if (teamPlayer.penalized || playingRoles_->playerRoles[teamPlayer.playerNumber - 1] != PlayingRole::NONE)
    {
      continue;
    }
    const float ttrb = actualTimeToReachBall(teamPlayer.playerNumber, cycleInfo_->getTimeDiff(teamPlayer.timeWhenReachBall),
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

  // 4. Determine which other roles should be taken.
  selectRemainingPlayerRoles();

  // 5. Overwrite the own role if an eligible role provider has one for us.
  if (useTeamRole_() || gameControllerState_->state != GameState::PLAYING)
  {
    unsigned int minPlayerNumber = playerConfiguration_->playerNumber;
    for (auto& teamPlayer : teamPlayers_->players)
    {
      if (!teamPlayer.penalized && teamPlayer.playerNumber < minPlayerNumber && teamPlayer.roleAssignments.size() >= playerConfiguration_->playerNumber &&
          teamPlayer.roleAssignments[playerConfiguration_->playerNumber - 1] != PlayingRole::NONE)
      {
        minPlayerNumber = teamPlayer.playerNumber;
        playingRoles_->role = teamPlayer.roleAssignments[playerConfiguration_->playerNumber - 1];
      }
    }
  }

  // 6. Set last assignment (for hysteresis).
  lastAssignment_ = playingRoles_->playerRoles;
}

void PRP::selectRemainingPlayerRoles()
{
  std::vector<Player> remainingPlayers;
  if (playingRoles_->playerRoles[playerConfiguration_->playerNumber - 1] == PlayingRole::NONE)
  {
    remainingPlayers.emplace_back(playerConfiguration_->playerNumber, robotPosition_->pose.position.x());
  }
  for (auto& teamPlayer : teamPlayers_->players)
  {
    if (teamPlayer.penalized || playingRoles_->playerRoles[teamPlayer.playerNumber - 1] != PlayingRole::NONE)
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
  std::sort(remainingPlayers.begin(), remainingPlayers.end(), [](const Player& p1, const Player& p2) { return p1.x < p2.x; });
  if (remainingPlayers.size() == 2)
  {
    // Of two remaining field players one should be defender and the other should be supporter.
    updateRole(remainingPlayers[0].playerNumber, PlayingRole::DEFENDER);
    updateRole(remainingPlayers[1].playerNumber, PlayingRole::SUPPORT_STRIKER);
  }
  else if (remainingPlayers.size() == 3)
  {
    // This is the maximum situation in normal games.
    // Two robots should be defender and one should be supporter or bishop.
    // The bishop/supporter decision depends on how far the ball is in the opponent's half.
    updateRole(remainingPlayers[0].playerNumber, PlayingRole::DEFENDER);
    updateRole(remainingPlayers[1].playerNumber, PlayingRole::DEFENDER);

    // If the ball is far from the own goal, a bishop is useful because the two defenders can
    // take the supporting role of catching lost striker balls and the bishop can take balls that go into the opponent's half.
    // On the other hand, when the ball is near the opponent's goal, no bishop is needed anymore and the defenders are far
    // from the ball so there should be a supporter that collects balls that are lost by the striker.
    bool hadBishop = false;
    for (auto& player : remainingPlayers)
    {
      // We only want the hadBishop bonus if the same robot would become bishop again.
      if (lastRoleOf(player.playerNumber) == PlayingRole::BISHOP && player.playerNumber == remainingPlayers[2].playerNumber)
      {
        hadBishop = true;
        break;
      }
    }
    if ((teamBallModel_->ballType != TeamBallModel::BallType::NONE) ? (teamBallModel_->position.x() < (hadBishop ? 2.f : 1.f)) : hadBishop)
    {
      updateRole(remainingPlayers[2].playerNumber, PlayingRole::BISHOP);
    }
    else
    {
      updateRole(remainingPlayers[2].playerNumber, PlayingRole::SUPPORT_STRIKER);
    }
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
  throw std::runtime_error("Unknown PlayingRole '" + configRole + "'!");
}

float PRP::actualTimeToReachBall(const unsigned int playerNumber, const float timeToReachBall, const float timeToReachBallStriker) const
{
  if (playingRoles_->playerRoles[playerNumber - 1] != PlayingRole::NONE)
  {
    return std::numeric_limits<float>::max();
  }
  const bool wasStriker = lastRoleOf(playerNumber) == PlayingRole::STRIKER;
  return wasStriker ? timeToReachBallStriker : timeToReachBall;
}

PlayingRole PRP::lastRoleOf(const unsigned int playerNumber) const
{
  return playerNumber <= lastAssignment_.size() ? lastAssignment_[playerNumber - 1] : PlayingRole::NONE;
}
