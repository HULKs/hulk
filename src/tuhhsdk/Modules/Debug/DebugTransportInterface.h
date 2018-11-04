#pragma once

#include <boost/variant.hpp>
#include <string>

#include "DebugData.h"

/**
 * @brief DebugTransportInterface the interface for all *Transport classes.
 */
class DebugTransportInterface
{
public:
  /**
   * @brief destructor
   */
  virtual ~DebugTransportInterface() {}
  /**
   * @brief hook that is called after a debug source's cycle.
   * Can be used to send/write debug data that was collected during this cycle.
   */
  virtual void transport() = 0;
};
