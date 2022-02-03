#pragma once

#include <cstdint>
#include <memory>


class Configuration;

class NetworkConfig
{
private:
  class Impl;

  std::shared_ptr<Impl> pimpl_;

public:
  NetworkConfig(const std::uint16_t& port, Configuration& config);
  ~NetworkConfig();

  void run();
};
