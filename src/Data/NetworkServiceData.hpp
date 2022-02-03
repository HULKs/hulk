#pragma once

#include "Framework/DataType.hpp"


class NetworkServiceData : public DataType<NetworkServiceData>
{
public:
  /**
   * @brief NetworkInterface A struct for storing information about a single network interface in.
   */
  struct NetworkInterface : public Uni::To
  {
    std::string name;
    uint32_t address;
    std::array<char, 4> addressArray;
    std::string addressString;
    std::string essid;

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["name"] << name;
      value["address"] << address;
      value["addressArray"] << addressArray;
      value["addressString"] << addressString;
      value["essid"] << essid;
    }
  };

  DataTypeName name__{"NetworkServiceData"};
  /// Whether the interfaces are up to date and considered reliable. Can only be true on a NAO.
  bool valid;
  /// All interfaces that were found on this robot.
  std::vector<NetworkInterface> interfaces;

  /// whether there is any active interface with a non empty connected ESSID
  bool isConnectedToAnyWifi;
  /// whether there is any active interface with "eth"/"ETH" in it's name
  bool isConnectedToAnyEth;

  void reset() override
  {
    // interfaces are not being reset on purpose
    valid = false;
    isConnectedToAnyEth = false;
    isConnectedToAnyWifi = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["interfaces"] << interfaces;
    value["isConnectedToAnyWifi"] << isConnectedToAnyWifi;
    value["isConnectedToAnyEth"] << isConnectedToAnyEth;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value&) override
  {
    // not implemented
  }
};
