#pragma once

#include <boost/asio/ip/address.hpp>
#include <functional>
#include <vector>

#include "Definitions/SPLStandardMessage.h"
#include "Framework/DataType.hpp"
#include "Modules/Debug/Debug.h"
#include "Tools/Time.hpp"


class SPLNetworkData : public DataType<SPLNetworkData>
{
public:
  struct IncomingMessage
  {
    /**
     * @brief IncomingMessage initializes all fields of this struct
     *
     * @param msg the spl standard message that was received
     * @param timePoint the timepoint when the message arrived
     * @param addr the address of the sender
     */
    IncomingMessage(const SPLStandardMessage& msg, const TimePoint& timePoint,
                    const boost::asio::ip::address& addr)
      : stdMsg(msg)
      , receiveTimePoint(timePoint)
      , senderAddr(addr)
    {
    }

    /// The message that was received
    SPLStandardMessage stdMsg;
    /// The timepoint when this message arrived
    TimePoint receiveTimePoint;
    /// The origin of this message
    boost::asio::ip::address senderAddr;
  };

  /// the name of this DataType
  DataTypeName name = "SPLNetworkData";
  /// SPL messages that arrived during the last cycle
  std::vector<IncomingMessage> messages;
  /// a function for sending messages
  // This is a function handle because the details of message sending should be hidden inside the
  // SPLNetworkService.
  std::function<void(const SPLStandardMessage&)> sendMessage;
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

  void fromValue(const Uni::Value&) override
  {
    // Nothing in here...
  }
};
