#pragma once

#include "Framework/Module.hpp"

#include "Data/CycleInfo.hpp"
#include "Data/NetworkServiceData.hpp"

class Brain;

/**
 * @brief NetworkService provides information about the network interfaces
 */
class NetworkService : public Module<NetworkService, Brain>
{
public:
  ModuleName name__{"NetworkService"};

  /**
   * @brief NetworkService initializes members
   * @param manager a reference to brain
   */
  explicit NetworkService(const ModuleManagerInterface& manager);

  ~NetworkService() override;

  void cycle() override;

private:
  /// CycleInfo is used for timing checks
  const Dependency<CycleInfo> cycleInfo_;

  Production<NetworkServiceData> networkServiceData_;

  /// the last time we queried for new interfaces
  Clock::time_point lastTimeQueried_;
  /// whether the last updateInterface() was successful
  bool lastUpdateValid_;
  /// whether we are connected to a cable based network
  bool isConnectedToAnyEth_;
  /// whether we are connected to a wireless network
  bool isConnectedToAnyWifi_;
  /// The socket to call ioctls on
  int socketFd_;

  /**
   * @brief updateInterfaces refreshes the interface list in networkServiceData
   * @return whether the operation was successful
   */
  bool updateInterfaces();

  /**
   * @brief getConnectedESSID returns the connected essid of the given interface (if any)
   * @param interface the interface to get the essid for
   * @return the essid as string. Empty string on error or if interface is not wireless
   */
  std::string getConnectedESSID(const std::string& interface);
};
