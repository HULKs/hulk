#pragma once

#include "Framework/Debug/DebugTransportInterface.h"

#include <memory>
#include <string>
#include <unordered_map>


class Debug;
class Configuration;

typedef std::unordered_map<std::string, DebugData> DebugDataMap;

class FileTransport : public DebugTransportInterface
{
private:
  /**
   * @brief Impl the class with the actual implementation
   */
  class Impl;
  /// The pointer to the implementation
  std::unique_ptr<Impl> pimpl_;


public:
  /**
   * @brief FileTransport the constructor for filetransport
   * @param debug A reference to the TUHH global debug module
   * @param cfg A reference to the TUHH global configuration
   * @param filePath The file path where to save the filePath
   */
  FileTransport(Debug& debug, Configuration& cfg, const std::string& filePath);
  /**
   * @brief ~FileTransport the destructor for FileTransport objects
   */
  ~FileTransport() override;

  /**
   * @brief transport function that is periodically called after a debugSource has finished a cycle.
   */
  void transport() override;
};
