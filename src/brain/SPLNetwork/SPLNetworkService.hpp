#pragma once

#include <mutex>
#include <thread>
#include <vector>

#include <boost/array.hpp>
#include <boost/asio.hpp>
#include <boost/system/error_code.hpp>

#include "Data/PlayerConfiguration.hpp"
#include "Data/SPLNetworkData.hpp"
#include "Definitions/SPLStandardMessage.h"
#include "Framework/Module.hpp"


class Brain;

/**
 * Class to transmit and receive messages of the team members.
 *
 * @author Felix Patschkowski
 */
class SPLNetworkService : public Module<SPLNetworkService, Brain>
{
public:
  /// the name of this module
  ModuleName name = "SPLNetworkService";
  /**
   * @brief SPLNetworkService starts a networking thread
   * @param manager reference to brain
   */
  SPLNetworkService(const ModuleManagerInterface& manager);
  /**
   * @brief ~SPLNetworkService stops the networking thread
   */
  ~SPLNetworkService();
  /**
   * @brief cycle copies received messages to exposed list
   */
  void cycle();

private:
  /**
   * @brief registerForReceive registers the onSocketReceive method
   */
  void registerForReceive();
  /**
   * @brief onSocketReceive is called in the IO service thread when a message arrives
   * @param error an error code if there was an error
   * @param bytesTransferred the number of received bytes
   */
  void onSocketReceive(const boost::system::error_code& error, std::size_t bytesTransferred);
  /**
   * @brief sendMessage sends an SPL message asynchronously
   * @param message the message to send
   */
  void sendMessage(const SPLStandardMessage& message);
  /// whether multicast should be used so that SPL messages don't escape and invade from/to SimRobot
  const Parameter<bool> useMulticast_;
  /// player configuration is needed for the port
  const Dependency<PlayerConfiguration> playerConfiguration_;
  /// exports the sendMessage function and received messages
  Production<SPLNetworkData> splNetworkData_;
  /// internal list of messages
  std::vector<std::pair<SPLStandardMessage, TimePoint>> messages_;
  /// lock for the messages
  std::mutex lock_;
  /// an IO service that runs in a seperate thread
  boost::asio::io_service ioService_;
  /// the endpoint of the last incoming message
  boost::asio::ip::udp::endpoint lastSenderEndpoint_;
  /// the UDP endpoint to which packets are sent
  boost::asio::ip::udp::endpoint foreignEndpoint_;
  /// UDP network socket
  boost::asio::ip::udp::socket socket_;
  /// handle to background thread
  std::shared_ptr<std::thread> backgroundThread_;
  /// receive buffer
  boost::array<char, sizeof(SPLStandardMessage)> receive_;
  /// function handle for sendMessage
  std::function<void(const SPLStandardMessage&)> sendMessageHandle_;
};
