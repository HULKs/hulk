#include "Framework/Log/Log.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/FieldDimensionUtils.hpp"
#include "Tools/Math/Hysteresis.hpp"

#include "Brain/TeamPlayersAugmenter/TeamPlayersAugmenter.hpp"

TeamPlayersAugmenter::TeamPlayersAugmenter(const ModuleManagerInterface& manager)
  : Module(manager)
  , fieldDimensions_(*this)
  , rawTeamPlayers_(*this)
  , teamPlayers_(*this)
  , playerInOwnPenaltyArea_(6, false)
  , playerInOwnGoalBoxArea_(6, false)
{
}

void TeamPlayersAugmenter::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");

  // forward raw team players
  *teamPlayers_ = static_cast<TeamPlayers>(*rawTeamPlayers_);

  // set insideOwnPenaltyArea for each team player
  for (auto& player : teamPlayers_->players)
  {
    if (player.playerNumber <= playerInOwnPenaltyArea_.size() ||
        player.playerNumber <= playerInOwnGoalBoxArea_.size())
    {
      if (player.pose.x() < 0.f)
      {
        playerInOwnPenaltyArea_[player.playerNumber] = FieldDimensionUtils::isInPenaltyArea(
            player.pose.position(), fieldDimensions_, hysteresis_,
            playerInOwnPenaltyArea_[player.playerNumber]);
        playerInOwnGoalBoxArea_[player.playerNumber] = FieldDimensionUtils::isInGoalBoxArea(
            player.pose.position(), fieldDimensions_, hysteresis_,
            playerInOwnGoalBoxArea_[player.playerNumber]);
      }
      else
      {
        playerInOwnPenaltyArea_[player.playerNumber] = false;
        playerInOwnGoalBoxArea_[player.playerNumber] = false;
      }
      player.insideOwnPenaltyArea = playerInOwnPenaltyArea_[player.playerNumber];
      player.insideOwnGoalBoxArea = playerInOwnGoalBoxArea_[player.playerNumber];
    }
    else
    {
      player.insideOwnPenaltyArea = false;
      player.insideOwnGoalBoxArea = false;
      Log<M_BRAIN>(LogLevel::WARNING) << "In " << name__ << ": player number "
                                      << static_cast<int>(player.playerNumber) << " out of bounds!";
    }
    debug().update(mount_ + ".TeamPlayers", *teamPlayers_);
  }
}
