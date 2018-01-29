#pragma once

#include <boost/variant.hpp>
#include <string>

#include "DebugData.h"

class DebugTransportInterface
{
public:
  virtual void update(const DebugData& data) = 0;
  virtual void pushQueue(const std::string& key, const std::string& message) = 0;
  virtual void sendImage(const std::string& key, const Image& img) = 0;
  virtual void transport() = 0;
};
