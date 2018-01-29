#pragma once

#include <cstdint>
#include <mutex>
#include <thread>
#include <vector>

#include <boost/array.hpp>
#include <boost/asio.hpp>
#include <boost/system/error_code.hpp>

#include "Data/ButtonData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Definitions/RoboCupGameControlData.h"
#include "Framework/Module.hpp"
#include "Tools/Time.hpp"


typedef boost::array<char, sizeof(RoboCupGameControlData)> DataBuffer;
typedef boost::array<char, sizeof(RoboCupGameControlReturnData)> ReturnDataBuffer;

class Brain;

class GameController : public Module<GameController, Brain>
{
public:
  /**
   * @brief GameController starts the UDP message handler
   * @param manager a reference to brain
   */
  GameController(const ModuleManagerInterface& manager);
  /**
   * @brief ~GameController stops the UDP message handler
   */
  ~GameController();
  /**
   * @brief cycle handles the events that may have occured asynchronously and creates the GameControllerState
   */
  void cycle();

private:
  /**
   * @brief registerForSocketReceive
   */
  void registerForSocketReceive();
  /**
   * @brief sendReturnDataMessage sends a reply to the GameController, primarily to make it show up in the GameController UI
   * @param msg message type
   */
  void sendReturnDataMessage(uint8_t msg);
  /**
   * @brief onControlDataReceived is called when a new message arrived
   * @param data the received message
   * @return true iff the message was valid
   */
  bool onControlDataReceived(const RoboCupGameControlData& data);
  /**
   * @brief handleNetwork integrates GameController messages into the state
   */
  void handleNetwork();
  /**
   * @brief handleChestButton integrates chest button presses into the state
   */
  void handleChestButton();
  /// whether the game state should be overridden with penalty shootout when standing up in INITIAL
  const Parameter<bool> forcePenaltyShootout_;
  /// the team and player number configuration
  const Dependency<PlayerConfiguration> playerConfiguration_;
  /// the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// the button data
  const Dependency<ButtonData> buttonData_;
  /// state that is exposed to other modules
  Production<RawGameControllerState> rawGameControllerState_;
  /// internal state that is preserved across cycles
  RawGameControllerState internalState_;
  /// boost::array which stores received data
  DataBuffer receive_;
  /// boost::array which stores sent data
  ReturnDataBuffer send_;
  /// the thread in which the asio IO service runs
  std::shared_ptr<std::thread> backgroundThread_;
  /// boost::asio IO service that runs in its seperate thread
  boost::asio::io_service ioService_;
  /// UDP socket
  boost::asio::ip::udp::socket socket_;
  /// UDP endpoint for broadcast (messages to GameController)
  boost::asio::ip::udp::endpoint gameControllerEndpoint_;
  /// UDP endpoint of the last incoming packet (needed for return address)
  boost::asio::ip::udp::endpoint lastSenderEndpoint_;
  /// whether a message has already been received (i.e. whether lastSenderEndpoint_ is valid)
  bool receivedFromNetwork_;
  /// the last GameController message that has been received via the network
  RoboCupGameControlData latestData_;
  /// the timestamp of the latest GameController message receipt
  TimePoint latestDataTimestamp_;
  /// the timestamp of the last handled chest button double press
  TimePoint lastChestButtonDoublePress_;
  /// the timestamp of the last handled chest button single press
  TimePoint lastChestButtonSinglePress_;
  /// the index in the teams array of the RoboCupGameControlData
  unsigned int teamIndex_;
  /// mutex to prevent race conditions between the cycle and the asynchronous parts of this class
  std::mutex mutex_;
  /// whether new network data came in
  bool newNetworkData_;
};
