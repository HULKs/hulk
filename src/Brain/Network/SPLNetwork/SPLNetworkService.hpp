#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/SPLNetworkData.hpp"
#include "Framework/Module.hpp"
#include <boost/asio.hpp>
#include <mutex>
#include <thread>
#include <vector>

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
  ModuleName name__{"SPLNetworkService"};
  /**
   * @brief SPLNetworkService starts a networking thread
   * @param manager reference to brain
   */
  explicit SPLNetworkService(const ModuleManagerInterface& manager);
  /**
   * @brief ~SPLNetworkService stops the networking thread
   */
  ~SPLNetworkService() override;
  /**
   * @brief cycle copies received messages to exposed list
   */
  void cycle() override;

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
  void sendMessage(const SPLStandardMessage::SPLStandardMessage& message);
  /// whether multicast should be used so that SPL messages don't escape and invade from/to SimRobot
  const Parameter<bool> useMulticast_;
  /// player configuration is needed for the port
  const Dependency<PlayerConfiguration> playerConfiguration_;
  const Dependency<CycleInfo> cycleInfo_;
  /// exports the sendMessage function and received messages
  Production<SPLNetworkData> splNetworkData_;
  /// internal list of messages
  std::vector<SPLNetworkData::IncomingMessage> messages_;
  /// lock for the messages
  std::mutex mutex_;
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
  std::array<char, sizeof(SPLStandardMessage::SPLStandardMessage)> receive_;
  /// function handle for sendMessage
  std::function<void(const SPLStandardMessage::SPLStandardMessage&)> sendMessageHandle_;
};
