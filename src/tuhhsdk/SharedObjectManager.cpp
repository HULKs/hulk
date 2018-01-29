#include "Modules/Configuration/Configuration.h"

#include "print.h"

#include "SharedObjectManager.hpp"


SharedObjectManager::SharedObjectManager(Debug& debug, Configuration& config, RobotInterface& robotInterface)
  : debug_(debug)
  , config_(config)
  , robotInterface_(robotInterface)
  , loadedModules_()
  , conChannels_()
  , threadData_()
{
}

void SharedObjectManager::start()
{
  Log(LogLevel::INFO) << "Initializing shared objects";

  config_.mount("tuhhSDK.autoload", "tuhh_autoload.json", ConfigurationType::HEAD);

  Uni::Value& uvModules = config_.get("tuhhSDK.autoload", "modules");

  // A kn-Graph has n(n-1)/2 edges
  // So we need number of edges DuplexChannels for Messaging
  const size_t numVertices = uvModules.size();
  const size_t numEdges = (numVertices * (numVertices - 1)) / 2;
  threadData_.resize(numVertices);
  conChannels_.resize(numEdges);

  // Now we need to connect the Modules
  // An assignment for three nodes:
  //   1 2 3
  // 1(  1 2)
  // 2(    3)
  // 3(     )
  int assignedChannel = 0;
  for (size_t numFirst = 0; numFirst < numVertices; numFirst++)
  {
    for (size_t numSecond = numFirst + 1; numSecond < numVertices; numSecond++)
    {
      auto& t1 = threadData_[numFirst];
      auto& t2 = threadData_[numSecond];
      auto& d = conChannels_[assignedChannel];

      t1.senders.emplace_back(&d.getA2BSender());
      t2.receivers.emplace_back(&d.getA2BReceiver());
      t2.senders.emplace_back(&d.getB2ASender());
      t1.receivers.emplace_back(&d.getB2AReceiver());

      assignedChannel++;
    }
  }

  loadedModules_.reserve(uvModules.size());
  unsigned int numSharedObjects = 0;
  auto itS = uvModules.listBegin();
  auto itE = uvModules.listEnd();
  for(; itS != itE; itS++)
  {
    std::string sharedObject = (*itS)["sharedObject"].asString();
    std::string logl         = (*itS)["loglevel"].asString();

    ThreadData& tData = threadData_[numSharedObjects];
    tData.loglevel = getLogLevel(logl);
    tData.debug = &debug_;
    tData.configuration = &config_;
    tData.robotInterface = &robotInterface_;
    Log(LogLevel::INFO) << "Loading module \"" << sharedObject << "\" ...";

    try
    {
      loadedModules_.emplace_back(sharedObject, tData);
      Log(LogLevel::INFO) << "... Success";
    }
    catch (const std::exception& e)
    {
      Log(LogLevel::ERROR) << e.what();
      throw;
    }
    numSharedObjects++;
  }
  for (auto& module : loadedModules_)
  {
    module.start();
  }
}

void SharedObjectManager::stop()
{
  for (auto& module : loadedModules_)
  {
    module.stop();
  }
  for (auto& module : loadedModules_)
  {
    module.join();
  }
  loadedModules_.clear();
  threadData_.clear();
  conChannels_.clear();
}
