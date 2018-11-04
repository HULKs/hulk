#pragma once

#include <vector>
#include <functional>

#include "Tools/Time.hpp"
#include "Definitions/SPLStandardMessage.h"
#include "Framework/DataType.hpp"
#include "Modules/Debug/Debug.h"


class SPLNetworkData : public DataType<SPLNetworkData> {
public:
  /// the name of this DataType
  DataTypeName name = "SPLNetworkData";
  /// SPL messages that arrived during the last cycle
  std::vector<std::pair<SPLStandardMessage, TimePoint>> messages;
  /// a function for sending messages
  // This is a function handle because the details of message sending should be hidden inside the SPLNetworkService.
  std::function<void(const SPLStandardMessage&)> sendMessage;
  /**
   * @brief reset clears the list of messages
   */
  void reset()
  {
    messages.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    // Nothing in here...
  }

  virtual void fromValue(const Uni::Value&)
  {
    // Nothing in here...
  }
};
