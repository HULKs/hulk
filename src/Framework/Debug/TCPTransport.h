#pragma once

#include "Framework/Debug/DebugTransportInterface.h"
#include "Tools/Var/SpscQueue.hpp"

#include <cstdint>
#include <memory>


class Debug;

/**
 * @brief TCPTransport is a transporter for sending debug values via network (tcp)
 */
class TCPTransport : public DebugTransportInterface
{
private:
  class Impl;
  class Session;

  /// pointer to implementation
  std::unique_ptr<Impl> pimpl_;

public:
  /**
   * @brief TCPTransport initializes members
   * @param port the tcp port to use for sending debug data
   * @param debug a reference to debug (to get the transportable debugMap from)
   */
  TCPTransport(const std::uint16_t& port, Debug& debug);
  /**
   * @brief destructor
   */
  virtual ~TCPTransport();
  /**
   * @brief hook that should be called after a debug source's cycle.
   */
  void transport() override;
};
