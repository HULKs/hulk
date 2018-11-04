#pragma once

#include "Framework/Module.hpp"
#include "Tools/Time.hpp"

#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/NTPData.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/SPLNetworkData.hpp"
#include "Data/RawTeamPlayers.hpp"

class Brain;

class SPLMessageReceiver : public Module<SPLMessageReceiver, Brain>
{
public:
  /// the name of this module
  ModuleName name = "SPLMessageReceiver";
  /**
   * @brief SPLMessageReceiver initializes members
   * @param manager reference to brain
   */
  SPLMessageReceiver(const ModuleManagerInterface& manager);
  /**
   * @brief cycle integrates incoming messages into the list of players
   */
  void cycle();

private:
  // TODO: use a buffer of measurements and choose the offset with the smallest round trip time
  struct NTPRobot
  {
    /// whether an NTP measurement for this robot is valid
    bool valid = false;
    /// the offset of the other robot to this robot in ms
    int offset;
  };
  /// the own player number needs to be known
  const Dependency<PlayerConfiguration> playerConfiguration_;
  /// the incoming messages
  const Dependency<SPLNetworkData> splNetworkData_;
  /// the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// the game controller state
  const Dependency<RawGameControllerState> rawGameControllerState_;
  /// the exposed list of players
  Production<RawTeamPlayers> rawTeamPlayers_;
  /// the received NTP requests of this cycle
  Production<NTPData> ntpData_;
  /// the internal list of players
  RawTeamPlayers internalPlayers_;
  /// last time of cycle execution
  TimePoint lastTime_;
  /// a list of the robots which are known via NTP
  std::vector<NTPRobot> ntpRobots_;
};
