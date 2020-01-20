#pragma once

#include "Data/FieldDimensions.hpp"
#include "Data/TeamPlayers.hpp"
#include "Framework/Module.hpp"

class Brain;

class TeamPlayersAugmenter : public Module<TeamPlayersAugmenter, Brain>
{
public:
  ModuleName name = "TeamPlayersAugmenter";

  /**
   * @brief TeamPlayersAugmenter initializes members
   * @param manager reference to module manager interface
   */
  explicit TeamPlayersAugmenter(const ModuleManagerInterface& manager);

  /**
   * @brief cycle augments the TeamPlayers by adding information about whether a player is inside
   * the own penalty area
   */
  void cycle() override;

private:
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<RawTeamPlayers> rawTeamPlayers_;
  Production<TeamPlayers> teamPlayers_;

  const float hysteresis_ = 0.25f;
  // save state for hysteresis
  std::vector<bool> playerInOwnPenaltyArea_;
};
