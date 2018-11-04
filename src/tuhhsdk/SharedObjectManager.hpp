#pragma once

#include <vector>

#include "Framework/Messaging.hpp"
#include "Framework/Thread.hpp"

#include "SharedObject.hpp"


class Debug;
class Configuration;
class RobotInterface;

class SharedObjectManager final
{
public:
  /**
   * @brief SharedObjectManager initializes members
   * @param debug the Debug instance
   * @param config the Configuration instance
   * @param robotInterface the RobotInterface instance
   */
  SharedObjectManager(Debug& debug, Configuration& config, RobotInterface& robotInterface);
  /**
   * @brief start initializes shared objects from tuhh_autoload.json file and starts threads
   */
  void start();
  /**
   * @brief stop stops all shared objects and makes sure nothing of them runs after this method
   */
  void stop();

private:
  /*
   * @brief checkAllRequestedDataTypes checks if all requested DataTypes are satisfied by other ModuleManagers
   */
  void checkAllRequestedDataTypes();
  /// a reference to the Debug instance
  Debug& debug_;
  /// a reference to the Configuration instance
  Configuration& config_;
  /// a reference to the RobotInterface instance
  RobotInterface& robotInterface_;
  /// all loaded shared objects
  std::vector<SharedObject> loadedSharedObjects_;
  /// the connection channels between shared objects
  std::vector<DuplexChannel> conChannels_;
  /// additional data for each thread
  std::vector<ThreadData> threadData_;
};
