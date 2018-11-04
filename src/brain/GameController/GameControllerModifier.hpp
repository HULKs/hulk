#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/WhistleData.hpp"
#include "Framework/Module.hpp"
#include "Tools/Time.hpp"


class Brain;

class GameControllerModifier : public Module<GameControllerModifier, Brain>
{
public:
  /// the name of this module
  ModuleName name = "GameControllerModifier";
  /**
   * @brief GameControllerModifier integrates internal knowledge to the game controller state
   * @param manager a reference to brain
   */
  GameControllerModifier(const ModuleManagerInterface& manager);

  void cycle();

private:
  /**
   * @brief integrates the whistle into the gameControllerState
   */
  void integrateWhistle();

  /// If a heard whistle should be able modify the game state.
  const Parameter<bool> enableWhistleIntegration_;
  /// Number of robots that need to have heard the whistle in SET to change to PLAYING
  const Parameter<unsigned int> minNumOfDetectedWhistles_;

  /// game controller state from network and chest button
  const Dependency<RawGameControllerState> rawGameControllerState_;
  /// whether the whistle was detected in this cycle
  const Dependency<WhistleData> whistleData_;
  /// the active players of the own team
  const Dependency<TeamPlayers> teamPlayers_;
  /// the cycle info
  const Dependency<CycleInfo> cycleInfo_;

  /// The final gameControllerState as given to other modules
  Production<GameControllerState> gameControllerState_;

  /// The rawGameControllerState of the last cycle
  RawGameControllerState prevRawGameControllerState_;
  /// The gameControllerState of the last cycle (this module's production)
  GameControllerState prevGameControllerState_;
  /// time when the gameControllerState changed last.
  TimePoint stateChanged_;
  /// time when the nao started listening for a whistle
  TimePoint lastTimeStartedWhistleDetection_;
};
