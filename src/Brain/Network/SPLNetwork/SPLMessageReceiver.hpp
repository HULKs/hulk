#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/NTPData.hpp"
#include "Data/NetworkServiceData.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/RawTeamPlayers.hpp"
#include "Data/SPLNetworkData.hpp"
#include "Framework/Module.hpp"
#include "Hardware/Clock.hpp"
#include <chrono>

class Brain;

class SPLMessageReceiver : public Module<SPLMessageReceiver, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"SPLMessageReceiver"};
  /**
   * @brief SPLMessageReceiver initializes members
   * @param manager reference to brain
   */
  explicit SPLMessageReceiver(const ModuleManagerInterface& manager);
  /**
   * @brief cycle integrates incoming messages into the list of players
   */
  void cycle() override;

private:
  // TODO: use a buffer of measurements and choose the offset with the smallest round trip time
  struct NTPRobot
  {
    /// whether an NTP measurement for this robot is valid
    bool valid = false;
    /// the offset of the other robot to this robot in ms
    int offset;
  };
  /// whether this module should play an acustic warning about same player numbers in the network
  const Parameter<bool> enablePlayerNumberWarning_;
  /// the own player number needs to be known
  const Dependency<PlayerConfiguration> playerConfiguration_;
  /// information about the network interfaces
  const Dependency<NetworkServiceData> networkServiceData_;
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
  Clock::time_point lastTime_;
#ifdef HULK_TARGET_NAO
  /// a list of the robots which are known via NTP
  std::vector<NTPRobot> ntpRobots_;
#endif

  /**
   * @brief parseDSMsg tries to extract the DSmsg from msg.data and writes all information into
   * player
   *
   * @param msg the received standard message
   * @param remainingBytes the number of bytes that were not parsed yet
   * @param receivedSystemTimePoint the time when the message was received (in system time)
   * @param player the player object to write the parsed data into
   * @return the number of bytes that were parsed (0 on failure)
   */
  unsigned int parseDSMsg(const SPLStandardMessage::SPLStandardMessage& msg,
                          unsigned int remainingBytes,
                          const std::chrono::steady_clock::time_point& receivedSystemTimePoint,
                          RawTeamPlayer& player);

  /**
   * @brief parseHULKMsg tries to extract the HULKmsg from msg data and writes all info into player
   *
   * @param msg the received standard message
   * @param remainingBytes the number of bytes that were not parsed yet
   * @param player the player object to write the parsed data into
   * @return the number of bytes that were parsed (0 on failure)
   */
  unsigned int parseHULKMsg(const SPLStandardMessage::SPLStandardMessage& msg,
                            unsigned int remainingBytes, RawTeamPlayer& player);
};
