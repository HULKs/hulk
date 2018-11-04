#include <algorithm>
#include <cassert>
#include <limits>
#include <stdexcept>

#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Random.hpp"

#include "SetPositionProvider.hpp"


SetPositionProvider::SetPositionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , keeperPosition_(*this, "keeperPosition")
  , defensivePositions_(*this, "defensivePositions")
  , offensivePositions_(*this, "offensivePositions")
  , considerRole_(*this, "considerRole")
  , enableDribbleAroundOpponentAtKickoff_(*this, "enableDribbleAroundOpponentAtKickoff")
  , dribbleAngle_(*this, "dribbleAngle", [this] { dribbleAngle_() *= TO_RAD; })
  , kickoffDribbleSign_(0.f)
  , cycleInfo_(*this)
  , gameControllerState_(*this)
  , playerConfiguration_(*this)
  , playingRoles_(*this)
  , robotPosition_(*this)
  , teamPlayers_(*this)
  , setPosition_(*this)
{
  dribbleAngle_() *= TO_RAD;
  // At least the kickoff position and one field position need to be given for both defensive and
  // offensive line-ups.
  if (defensivePositions_().size() < 2 || offensivePositions_().size() < 2)
  {
    throw std::runtime_error("SetPositionProvider: defensivePositions and offensivePositions must "
                             "contain at least two elements!");
  }
}

void SetPositionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  // reset sign if ready state has just begun
  if (cycleInfo_->getTimeDiff(gameControllerState_->gameStateChanged) < 1.f &&
      gameControllerState_->gameState == GameState::READY && kickoffDribbleSign_ != 0.f)
  {
    kickoffDribbleSign_ = 0.f;
  }

  // A SET position is only needed during READY and SET (actually only during READY).
  if (gameControllerState_->gameState != GameState::READY &&
      gameControllerState_->gameState != GameState::SET)
  {
    return;
  }

  if ((!considerRole_() && playerConfiguration_->playerNumber == 1) ||
      (considerRole_() && playingRoles_->role == PlayingRole::KEEPER))
  {
    // The player with number 1 is statically assigned the keeper position.
    setPosition_->position = keeperPosition_();
  }
  else
  {
    // First, the player closest to the kickoff position (the first one in the array) is determined.
    const auto& positions =
        gameControllerState_->kickingTeam ? offensivePositions_() : defensivePositions_();
    // We actually need to know which player is nearest (and not only whether we are closest)
    // because it needs to be excluded from the remaining players.
    unsigned int kickoffStriker = playerConfiguration_->playerNumber;
    if (!considerRole_())
    {
      float minDistance = (robotPosition_->pose.position - positions[0]).squaredNorm();
      for (auto& teamPlayer : teamPlayers_->players)
      {
        if (teamPlayer.penalized || teamPlayer.playerNumber == 1)
        {
          continue;
        }
        const float distance = (teamPlayer.pose.position - positions[0]).squaredNorm();
        if (distance < minDistance)
        {
          kickoffStriker = teamPlayer.playerNumber;
          minDistance = distance;
        }
      }
    }
    else
    {
      if (playingRoles_->role == PlayingRole::STRIKER)
      {
        kickoffStriker = playerConfiguration_->playerNumber;
      }
      else
      {
        for (auto& teamPlayer : teamPlayers_->players)
        {
          if (teamPlayer.currentlyPerformingRole == PlayingRole::STRIKER)
          {
            kickoffStriker = teamPlayer.playerNumber;
            break;
          }
        }
      }
    }
    if (kickoffStriker == playerConfiguration_->playerNumber)
    {
      Vector2f kickoffStrikerPosition = positions[0];
      // If dribbling around opponent is enabled the kickoff position is either slightly either to
      // the left or right.
      if (gameControllerState_->kickingTeam && enableDribbleAroundOpponentAtKickoff_())
      {
        if (kickoffDribbleSign_ == 0)
        {
          kickoffDribbleSign_ = Random::uniformInt(0, 1) == 0 ? -1.f : 1.f;
        }
        // rotate the kickoff striker position by dribble angle and use sign for left/right decision
        kickoffStrikerPosition =
            Rotation2Df(kickoffDribbleSign_ * dribbleAngle_()) * kickoffStrikerPosition;
        ;
      }
      setPosition_->position = kickoffStrikerPosition;
      setPosition_->isKickoffPosition = true;
    }
    else
    {
      // If this robot is neither keeper nor kickoff striker, its position is calculated so that
      // the overall squared distance for all remaining players in the team is minimal.
      // It is assumed that all other players do the same calculations and will come to the same
      // results.
      VecVector2f remainingTeamPlayers;
      for (auto& teamPlayer : teamPlayers_->players)
      {
        // If there are more field players than positions, some will be double-occupied.
        // Yes, this is random, but there seems to be no better choice in that case.
        // The + 2 is there to include the kickoff striker (which is always another robot at this
        // point) and the own robot.
        if (remainingTeamPlayers.size() + 2 == positions.size())
        {
          break;
        }
        if (teamPlayer.penalized || (!considerRole_() && teamPlayer.playerNumber == 1) ||
            (considerRole_() && teamPlayer.currentlyPerformingRole == PlayingRole::KEEPER) ||
            teamPlayer.playerNumber == kickoffStriker)
        {
          continue;
        }
        remainingTeamPlayers.push_back(teamPlayer.pose.position);
      }
      Vector2f myBestPosition = positions[1];
      float bestValue = std::numeric_limits<float>::max();
      // Create the lexicographically smallest permutation as initial assignment.
      std::vector<unsigned int> currentPerm(remainingTeamPlayers.size() + 1);
      for (unsigned int i = 0; i < currentPerm.size(); i++)
      {
        currentPerm[i] = i + 1;
      }
      // Go through all possible assignments from players to positions.
      // Even in a Mixed Team game with 4 not-kickoff-striker field players these are only 24.
      do
      {
        // It is also checked whether the assignment of the players to mirrored positions (regarding
        // the x-axis) would be better.
        float value = getPermutationValue(currentPerm, positions, remainingTeamPlayers, 1.f);
        float mirroredValue =
            getPermutationValue(currentPerm, positions, remainingTeamPlayers, -1.f);
        // It is not that improbable that value and mirroredValue are identical.
        // This will happen e.g. always when all positions can be occupied.
        if (value <= mirroredValue && value < bestValue)
        {
          bestValue = value;
          myBestPosition = positions[currentPerm[0]];
        }
        else if (mirroredValue < value && mirroredValue < bestValue)
        {
          bestValue = mirroredValue;
          myBestPosition = positions[currentPerm[0]];
          myBestPosition.y() = -myBestPosition.y();
        }
      } while (std::next_permutation(currentPerm.begin(), currentPerm.end()));
      setPosition_->position = myBestPosition;
    }
  }
  setPosition_->valid = true;
}

float SetPositionProvider::getPermutationValue(const std::vector<unsigned int>& perm,
                                               const VecVector2f& positions,
                                               const VecVector2f& remainingTeamPlayers,
                                               const float signY) const
{
  assert(perm.size() == remainingTeamPlayers.size() + 1);
  assert(positions.size() > perm.size());
  if (considerRole_())
  {
    // Check if this permutation would assign a defender a non-defender position and return maximum
    // (worst) score in that case.
    if (!roleIsCompatibleWithPosition(playingRoles_->role, perm[0]))
    {
      return std::numeric_limits<float>::max();
    }
    // If consider role is true, the remaining team players do not contain the kickoff striker and
    // the keeper (even though there may be two strikers).
    unsigned int remainingTeamPlayersIndex = 1;
    bool haveStrikerYet = false;
    for (auto& teamPlayer : teamPlayers_->players)
    {
      if (teamPlayer.penalized || teamPlayer.currentlyPerformingRole == PlayingRole::KEEPER ||
          (teamPlayer.currentlyPerformingRole == PlayingRole::STRIKER && !haveStrikerYet))
      {
        haveStrikerYet |= teamPlayer.currentlyPerformingRole == PlayingRole::STRIKER;
        continue;
      }
      assert(remainingTeamPlayersIndex < perm.size());
      if (!roleIsCompatibleWithPosition(teamPlayer.currentlyPerformingRole,
                                        perm[remainingTeamPlayersIndex]))
      {
        return std::numeric_limits<float>::max();
      }
      remainingTeamPlayersIndex++;
    }
  }
  float sum = (robotPosition_->pose.position -
               Vector2f(positions[perm[0]].x(), signY * positions[perm[0]].y()))
                  .squaredNorm();
  for (unsigned int i = 0; i < remainingTeamPlayers.size(); i++)
  {
    sum += (remainingTeamPlayers[i] -
            Vector2f(positions[perm[i + 1]].x(), signY * positions[perm[i + 1]].y()))
               .squaredNorm();
  }
  return sum;
}

bool SetPositionProvider::roleIsCompatibleWithPosition(const PlayingRole role,
                                                       const unsigned int posIndex) const
{
  // Defenders must get one of the first two non-striker positions.
  // This is currently only valid for the mixed team setup because the defender positions are the
  // first two non-striker positions there.
  return !(role == PlayingRole::DEFENDER && posIndex >= 3);
}
