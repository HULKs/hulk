#pragma once

#include "Framework/DataType.hpp"


class NTPData : public DataType<NTPData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"NTPData"};
  struct NTPRequest : public Uni::From, public Uni::To
  {
    /// the player number of the request sender
    unsigned int sender;
    /// the timestamp of the sender at which the message has been sent [ms]
    unsigned int origination;
    /// the timestamp of the receiver at which the message has been received [ms]
    unsigned int receipt;

    void toValue(Uni::Value& out) const override
    {
      out = Uni::Value(Uni::ValueType::OBJECT);
      out["sender"] << sender;
      out["origination"] << origination;
      out["receipt"] << receipt;
    }
    void fromValue(const Uni::Value& in) override
    {
      in["sender"] >> sender;
      in["origination"] >> origination;
      in["receipt"] >> receipt;
    }
  };
  /// a list of all NTP requests that have been received
  std::vector<NTPRequest> ntpRequests;
  /**
   * ·@brief reset clears the incoming NTP requests
   */
  void reset() override
  {
    ntpRequests.clear();
  }

  void toValue(Uni::Value& out) const override
  {
    out = Uni::Value(Uni::ValueType::OBJECT);
    out["ntpRequests"] << ntpRequests;
  }
  void fromValue(const Uni::Value& in) override
  {
    in["ntpRequests"] >> ntpRequests;
  }
};
