#pragma once

#include <cstdint>
#include <memory>

#include "Modules/Debug/DebugTransportInterface.h"

#ifndef _WIN32

class Debug;

class UnixSocketTransport : public DebugTransportInterface
{
private:
  class Impl;
  class Session;

  std::unique_ptr<Impl> pimpl_;

public:
  UnixSocketTransport(const std::string& file, Debug& debug);
  ~UnixSocketTransport();

  virtual void update(const DebugData& data);
  virtual void pushQueue(const std::string& key, const std::string& message);
  virtual void sendImage(const std::string& key, const Image& img);
  virtual void transport();
};

#endif
