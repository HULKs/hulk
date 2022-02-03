#pragma once

#include "Framework/DataType.hpp"
#include "Framework/Debug/Debug.h"
#include "Hardware/Clock.hpp"
#include "Messages/SPLStandardMessage.hpp"
#include <boost/asio/ip/address.hpp>
#include <chrono>
#include <functional>
#include <vector>


class SPLNetworkData : public DataType<SPLNetworkData>
{
public:
  struct IncomingMessage
  {
    /**
     * @brief IncomingMessage initializes all fields of this struct
     *
     * @param message the spl standard message that was received
     * @param systemTimePoint the timepoint when the message arrived (in system time)
     * @param address the address of the sender
     */
    IncomingMessage(const SPLStandardMessage::SPLStandardMessage& message,
                    const std::chrono::steady_clock::time_point& systemTimePoint,
                    boost::asio::ip::address address)
      : message{message}
      , receivedSystemTimePoint{systemTimePoint}
      , senderAddress{std::move(address)}
    {
    }

    /// The message that was received
    SPLStandardMessage::SPLStandardMessage message;
    /// The timepoint when this message arrived (in system time)
    std::chrono::steady_clock::time_point receivedSystemTimePoint;
    /// The origin of this message
    boost::asio::ip::address senderAddress;
  };

  /// the name of this DataType
  DataTypeName name__{"SPLNetworkData"};
  /// SPL messages that arrived during the last cycle
  std::vector<IncomingMessage> messages;
  /// a function for sending messages
  // This is a function handle because the details of message sending should be hidden inside the
  // SPLNetworkService.
  std::function<void(const SPLStandardMessage::SPLStandardMessage&)> sendMessage;
  /**
   * @brief reset clears the list of messages
   */
  void reset() override
  {
    messages.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    // Nothing in here...
  }

  void fromValue([[maybe_unused]] const Uni::Value& value) override
  {
    // Nothing in here...
  }
};
