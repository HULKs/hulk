#pragma once

#include <Modules/Debug/DebugTransportInterface.h>
#include <Tools/Var/SpscQueue.hpp>

#include <cstdint>
#include <memory>


class Debug;

class TCPTransport : public DebugTransportInterface
{
private:
  class Impl;
  class Session;

  std::unique_ptr<Impl> pimpl_;

public:
  TCPTransport(const std::uint16_t& port, Debug& debug);
  ~TCPTransport();

  virtual void update(const DebugData& data);
  virtual void pushQueue(const std::string& key, const std::string& message);
  virtual void sendImage(const std::string& key, const Image& img);
  virtual void transport();
  void run();
};
