#include "Brain/Network/NetworkService.hpp"

#ifdef HULK_TARGET_NAO
#include <cerrno>
#include <cstring>
#include <ifaddrs.h>
#include <netdb.h>
#endif

#include "Framework/Log/Log.hpp"
#include "Tools/Chronometer.hpp"
#include <linux/wireless.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

NetworkService::NetworkService(const ModuleManagerInterface& manager)
  : Module(manager)
  , cycleInfo_(*this)
  , networkServiceData_(*this)
  , lastTimeQueried_()
  , lastUpdateValid_(false)
  , isConnectedToAnyEth_(false)
  , isConnectedToAnyWifi_(false)
{
  socketFd_ = socket(AF_INET, SOCK_DGRAM, 0);
}

NetworkService::~NetworkService()
{
  close(socketFd_);
}

void NetworkService::cycle()
{
  // Network interfaces don't change that frequently. Check every n seconds only.
  if (cycleInfo_->getAbsoluteTimeDifference(lastTimeQueried_) < 1s)
  {
    networkServiceData_->valid = lastUpdateValid_;
    networkServiceData_->isConnectedToAnyEth = isConnectedToAnyEth_;
    networkServiceData_->isConnectedToAnyWifi = isConnectedToAnyWifi_;
    return;
  }

  {
    Chronometer time(debug(), mount_ + ".cycleTime");

    const bool wasConnectedToAnyEth = isConnectedToAnyEth_;
    const bool wasConnectedToAnyWifi = isConnectedToAnyWifi_;
    isConnectedToAnyEth_ = false;
    isConnectedToAnyWifi_ = false;

    networkServiceData_->valid = lastUpdateValid_ = updateInterfaces();
    lastTimeQueried_ = cycleInfo_->startTime; // Even set this if !valid to avoid flooding

    if (!networkServiceData_->valid)
    {
      return;
    }

    // check for any ethernet or wifi connection.
    for (const auto& interface : networkServiceData_->interfaces)
    {
      if (!interface.essid.empty())
      {
        isConnectedToAnyWifi_ = true;
        continue;
      }
      if (interface.name.find("eth") != std::string::npos ||
          interface.name.find("ETH") != std::string::npos)
      {
        isConnectedToAnyEth_ = true;
      }
    }

    networkServiceData_->isConnectedToAnyEth = isConnectedToAnyEth_;
    networkServiceData_->isConnectedToAnyWifi = isConnectedToAnyWifi_;

    if (isConnectedToAnyWifi_ != wasConnectedToAnyWifi)
    {
      Log<M_BRAIN>(LogLevel::INFO) << "WIFI interface changed state to "
                                   << (isConnectedToAnyWifi_ ? "CONNECTED" : "DISCONNECTED");
    }

    if (isConnectedToAnyEth_ != wasConnectedToAnyEth)
    {
      Log<M_BRAIN>(LogLevel::INFO) << "Ethernet interface changed state to "
                                   << (isConnectedToAnyEth_ ? "CONNECTED" : "DISCONNECTED");
    }
  }
}

bool NetworkService::updateInterfaces()
{
  networkServiceData_->interfaces.clear();

#ifndef HULK_TARGET_NAO
  return false;
#else
  struct ifaddrs* interfaceAddresses; // all interfaces

  // Try to get all interfaces
  if (getifaddrs(&interfaceAddresses) == -1)
  {
    std::array<char, 1024> errorString{};
    Log<M_BRAIN>(LogLevel::ERROR) << "Unable to get network interface information. Reason: "
                                  << strerror_r(errno, errorString.data(), errorString.size());
    return false;
  }

  // go through interfaces and get their corresponding IP address(es)
  for (auto ifaddr = interfaceAddresses; ifaddr != nullptr; ifaddr = ifaddr->ifa_next)
  {
    if (ifaddr->ifa_addr == nullptr)
    {
      continue;
    }

    // Skip unwanted families (ifaddr may also contain package stats and ipv6 addresses)
    const int family = ifaddr->ifa_addr->sa_family;
    if (family != AF_INET)
    {
      continue;
    }

    /**
     * @brief generateAddressRepresentations stores the given address as uint32_t, string and array
     * into the given NetworkInterface
     * @param addr the address to convert
     * @param iface the interface to store the representations into
     */
    const auto generateAddressRepresentations = [](uint32_t addr,
                                                   NetworkServiceData::NetworkInterface& iface) {
      iface.address = addr;
      auto* addrIt = reinterpret_cast<uint8_t*>(&addr);
      iface.addressString = "";
      for (int i = 0; i < 4; i++)
      {
        iface.addressArray[i] = addrIt[i];
        iface.addressString += std::to_string(addrIt[i]);
        if (i < 3)
        {
          iface.addressString += ".";
        }
      }
    };

    // get the address as one 32 bit value
    uint32_t addr = reinterpret_cast<sockaddr_in*>(ifaddr->ifa_addr)->sin_addr.s_addr;

    // check existing interfaces and merge them with the new one.
    bool found = false;
    for (auto& interface : networkServiceData_->interfaces)
    {
      if (interface.name == ifaddr->ifa_name)
      {
        found = true;
        generateAddressRepresentations(addr, interface);
        interface.essid = getConnectedESSID(interface.name);
      }
    }

    // new interface found, emplace it
    if (!found)
    {
      NetworkServiceData::NetworkInterface interface;
      interface.name = ifaddr->ifa_name;
      generateAddressRepresentations(addr, interface);
      interface.essid = getConnectedESSID(interface.name);
      networkServiceData_->interfaces.emplace_back(interface);
    }
  }

  freeifaddrs(interfaceAddresses);
  return true;
#endif
}

std::string NetworkService::getConnectedESSID(const std::string& interface)
{
  // based on:
  // http://papermint-designs.com/dmo-blog/2016-08-how-to-get-the-essid-of-the-wifi-network-you-are-connected-to-
  struct iwreq request;
  char essid[IW_ESSID_MAX_SIZE];

  if (interface.empty())
  {
    return std::string();
  }

  memcpy(request.ifr_ifrn.ifrn_name, interface.c_str(), IFNAMSIZ);
  memset(essid, 0, IW_ESSID_MAX_SIZE);
  request.u.essid.pointer = (caddr_t*)essid;
  request.u.data.length = IW_ESSID_MAX_SIZE;
  request.u.data.flags = 0;

  if (ioctl(socketFd_, SIOCGIWESSID, &request) < 0)
  {
    return std::string();
  }

  return std::string(essid);
}
