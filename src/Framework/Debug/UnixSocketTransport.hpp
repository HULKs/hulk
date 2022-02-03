#pragma once

#include <cstdint>
#include <memory>

#include "Framework/Debug/DebugTransportInterface.h"

class Debug;

/**
 * @brief UnixSocketTransport is a transporter for sending debug values via unix sockets
 */
class UnixSocketTransport : public DebugTransportInterface
{
private:
  class Impl;
  class Session;

  /// pointer to implementation
  std::unique_ptr<Impl> pimpl_;

public:
  /**
   * @brief UnixSocketTransport initializes members
   * @param file the file to write to
   * @param debug a reference to debug (to get the transportable debugMap from)
   */
  UnixSocketTransport(const std::string& file, Debug& debug);
  /**
   * @brief destructor
   */
  virtual ~UnixSocketTransport();
  /**
   * @brief hook that should be called after a debug source's cycle.
   */
  void transport() override;
};
