#pragma once

#include "Framework/Module.hpp"

#include "Data/GameControllerState.hpp"

/**
 * @brief GCAugmenterInterface a interface for game controller sub modules
 *
 * This interface is used for all game controller augmenter sub modules. The game controller
 * augmenter than manages these submodules and execute the given cycle method whenever needed.
 */
class GCAugmenterInterface
{
public:
  /**
   * @brief cycle integrates the whistle data into the given game controller state
   *
   * @param rawGcState the game controller state that was received from the network
   * @param gcState the game controller state that may be already modified by other sub modules
   */
  virtual void cycle(const RawGameControllerState& rawGcState, GameControllerState& gcState) = 0;
};
