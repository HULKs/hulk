#pragma once

#include "Framework/DataType.hpp"


class NTPData : public DataType<NTPData>
{
public:
  /// the name of this DataType
  DataTypeName name = "NTPData";
  struct NTPRequest : public Uni::From, public Uni::To
  {
    /// the player number of the request sender
    unsigned int sender;
    /// the timestamp of the sender at which the message has been sent [ms]
    unsigned int origination;
    /// the timestamp of the receiver at which the message has been received [ms]
    unsigned int receipt;

    virtual void toValue(Uni::Value& out) const
    {
      out = Uni::Value(Uni::ValueType::OBJECT);
      out["sender"] << sender;
      out["origination"] << origination;
      out["receipt"] << receipt;
    }
    virtual void fromValue(const Uni::Value& in)
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
  void reset()
  {
    ntpRequests.clear();
  }

  virtual void toValue(Uni::Value& out) const
  {
    out = Uni::Value(Uni::ValueType::OBJECT);
    out["ntpRequests"] << ntpRequests;
  }
  virtual void fromValue(const Uni::Value& in)
  {
    in["ntpRequests"] >> ntpRequests;
  }
};
