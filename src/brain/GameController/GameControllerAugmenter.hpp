#pragma once

#include "Framework/Module.hpp"

#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"

#include "augmenterModules/RefereeMistakeIntegration.hpp"
#include "augmenterModules/WhistleIntegration.hpp"


class Brain;

class GameControllerAugmenter : public Module<GameControllerAugmenter, Brain>
{
public:
  /// the name of this module
  ModuleName name = "GameControllerAugmenter";
  /**
   * @brief GameControllerAugmenter initializes members & submodules
   * @param manager a reference to brain
   */
  explicit GameControllerAugmenter(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  /// If a heard whistle should be able modify the game state.
  const Parameter<bool> enableWhistleIntegration_;
  /// If we want to correct obvious mistakes the refs made.
  const Parameter<bool> enableRefereeMistakeIntegration_;

  /// game controller state from network and chest button
  const Dependency<RawGameControllerState> rawGameControllerState_;
  /// the cycle info
  const Dependency<CycleInfo> cycleInfo_;

  /// The final gameControllerState as given to other modules
  Production<GameControllerState> gameControllerState_;

  /// The whistle integration sub module
  WhistleIntegration whistleIntegration_;
  /// The referee mistake integration sub module
  RefereeMistakeIntegration refereeMistakeIntegration_;
};
