#pragma once


#include "Framework/Module.hpp"

#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/WhistleData.hpp"

#include "GameController/GCAugmenterInterface.hpp"

/**
 * @brief WhistleIntegration integrates whistle data into the game controller state
 *
 * GC transition: SET -> Playing:
 *   - whenever the game controller state is SET all players are allowed to report a blown whistle
 *   - the players then check whether other robots also heard that whistle within a time frame.
 *   - If at least minNumOfDetectedWhistles_ players are agreeing the state changes to PLAYING.
 */
class WhistleIntegration : public GCAugmenterInterface
{
public:
  explicit WhistleIntegration(ModuleBase& module);

  void cycle(const RawGameControllerState& rawGcState, GameControllerState& gcState) override;

private:
  /**
   * @brief integrate knowledge from detected whistles into the given gc state
   *
   * @param rawGcState the raw game controller state as received via network
   * @param gcState the current gc state (may already be augmented by other sub modules
   */
  void integrateWhistle(const RawGameControllerState& rawGcState, GameControllerState& gcState);

  /// The maximum time diff (from "now" to lastTimeWhistleDetected) for a whistle timestamp to be
  /// valid. Unit: Seconds
  const Parameter<float> maxWhistleTimeDiff_;
  /// Number of robots that need to have heard the whistle in SET to change to PLAYING
  const Parameter<unsigned int> minNumOfDetectedWhistles_;

  /// Cycle info used to get the cycle's start time
  const Dependency<CycleInfo> cycleInfo_;
  /// whether the whistle was detected in this cycle
  const Dependency<WhistleData> whistleData_;
  /// the active players of the own team
  const Dependency<TeamPlayers> teamPlayers_;

  /// The previous raw game controller state used for detecting state transitions
  RawGameControllerState prevRawGcState_;
  /// The previous game controller state used for detecting state transitions
  /// Note that this is not necesserily the final production of the GCAugmenter
  GameControllerState prevGcState_;

  /// time when the gameControllerState changed last.
  TimePoint stateChanged_;
  /// time when the nao started listening for a whistle
  TimePoint lastTimeStartedWhistleDetection_;
};
