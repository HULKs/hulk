#pragma once

#include <memory>
#include <string>

#ifndef _WIN32

class Configuration;

class UnixSocketConfig
{
private:
  class Impl;

  std::shared_ptr<Impl> pimpl_;

public:
  UnixSocketConfig(const std::string& file, Configuration& config);
  ~UnixSocketConfig();

  void run();
};

#endif
