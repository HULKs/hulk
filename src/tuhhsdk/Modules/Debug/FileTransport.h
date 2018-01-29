#pragma once

#include <Modules/Debug/DebugTransportInterface.h>

#include <cstdint>
#include <memory>
#include <string>
#include <unordered_map>


class Debug;
class Configuration;

typedef std::unordered_map<std::string, DebugData> DebugDataMap;

class FileTransport : public DebugTransportInterface
{
private:
  class Impl;

  std::unique_ptr<Impl> pimpl_;


public:
  FileTransport(Debug& debug, Configuration& config, const std::string& fileRoot);
  ~FileTransport();

  virtual void update(const DebugData& data);
  virtual void pushQueue(const std::string& key, const std::string& message);
  virtual void sendImage(const std::string& key, const Image& img);
  virtual void transport();
};
