#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/WhistleData.hpp"
#include "Framework/Module.hpp"
#include "Tools/Time.hpp"


class Brain;

class WhistleIntegration : public Module<WhistleIntegration, Brain>
{
public:
  /**
   * @brief WhistleIntegration initializes members
   * @param manager a reference to brain
   */
  WhistleIntegration(const ModuleManagerInterface& manager);
  /**
   * @brief cycle overwrites the game state with PLAYING when the whistle is detected
   */
  void cycle();

private:
  /// the minimal number of robots which have to agree that the whistle has been heard
  const Parameter<unsigned int> minNumberOfAgreeingRobots_;
  /// game controller state from network and chest button
  const Dependency<RawGameControllerState> rawGameControllerState_;
  /// whether the whistle was detected in this cycle
  const Dependency<WhistleData> whistleData_;
  /// the active players of the own team
  const Dependency<TeamPlayers> teamPlayers_;
  /// the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// the corrected game controller state
  Production<GameControllerState> gameControllerState_;
  /// the game state of prev cycle
  GameState prevGameState_;
  /// the raw game state of prev cycle (i.e. without whistle override)
  GameState prevRawGameState_;
  /// the secondary state of prev cycle
  SecondaryState prevSecondaryState_;
  /// the time point at which the state has been changed due to the whistle
  TimePoint stateChanged_;
  /// the time point at which the last SET state has been entered
  TimePoint lastTimeOfSet_;
};
