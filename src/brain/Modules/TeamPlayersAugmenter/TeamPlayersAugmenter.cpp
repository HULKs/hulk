#include "Tools/Chronometer.hpp"
#include "Tools/Math/Hysteresis.hpp"
#include "Tools/PenaltyAreaUtils.hpp"
#include "print.h"

#include "TeamPlayersAugmenter.hpp"


TeamPlayersAugmenter::TeamPlayersAugmenter(const ModuleManagerInterface& manager)
  : Module(manager)
  , fieldDimensions_(*this)
  , rawTeamPlayers_(*this)
  , teamPlayers_(*this)
  , playerInOwnPenaltyArea_(6, false)
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
    if (player.playerNumber <= playerInOwnPenaltyArea_.size())
    {
      playerInOwnPenaltyArea_[player.playerNumber] =
          PenaltyAreaUtils::isInPenaltyArea(player.pose.position, fieldDimensions_, hysteresis_,
                                            playerInOwnPenaltyArea_[player.playerNumber]) &&
          (player.pose.position.x() < 0);
      player.insideOwnPenaltyArea = playerInOwnPenaltyArea_[player.playerNumber];
    }
    else
    {
      player.insideOwnPenaltyArea = false;
      Log(LogLevel::WARNING) << "In " << name << ": player number "
                             << static_cast<int>(player.playerNumber) << " out of bounds!";
    }
    debug().update(mount_ + ".TeamPlayers", *teamPlayers_);
  }
}
