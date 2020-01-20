#include "Modules/Configuration/Configuration.h"

#include "print.h"

#include "SharedObjectManager.hpp"


SharedObjectManager::SharedObjectManager(Debug& debug, Configuration& config, RobotInterface& robotInterface)
  : debug_(debug)
  , config_(config)
  , robotInterface_(robotInterface)
  , loadedSharedObjects_()
  , conChannels_()
  , threadData_()
{
}

void SharedObjectManager::start()
{
  Log(LogLevel::INFO) << "Initializing shared objects";

  config_.mount("tuhhSDK.autoload", "tuhh_autoload.json", ConfigurationType::HEAD);
  // load the module setups
  // first set the default config
  config_.mount("tuhhSDK.moduleSetup", "moduleSetup_default.json", ConfigurationType::HEAD);
  // overload this config with the more specific ones (similar to what is done with the locations)
  config_.mount("tuhhSDK.moduleSetup", "moduleSetup_" + config_.get("tuhhSDK.autoload", "moduleSetup").asString() + ".json", ConfigurationType::HEAD);

  Uni::Value& uvSharedObjects = config_.get("tuhhSDK.autoload", "sharedObjects");

  // A kn-Graph has n(n-1)/2 edges
  // So we need number of edges DuplexChannels for Messaging
  const size_t numVertices = uvSharedObjects.size();
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

  loadedSharedObjects_.reserve(uvSharedObjects.size());
  unsigned int numSharedObjects = 0;
  auto itS = uvSharedObjects.vectorBegin();
  auto itE = uvSharedObjects.vectorEnd();
  for (; itS != itE; itS++)
  {
    std::string sharedObject = (*itS)["sharedObject"].asString();
    std::string logl = (*itS)["loglevel"].asString();

    ThreadData& tData = threadData_[numSharedObjects];
    tData.loglevel = getLogLevel(logl);
    tData.debug = &debug_;
    tData.configuration = &config_;
    tData.robotInterface = &robotInterface_;
    Log(LogLevel::INFO) << "Loading sharedObject\"" << sharedObject << "\" ...";

    try
    {
      loadedSharedObjects_.emplace_back(sharedObject, tData);
      Log(LogLevel::INFO) << "... Success";
    }
    catch (const std::exception& e)
    {
      Log(LogLevel::ERROR) << e.what();
      throw;
    }
    numSharedObjects++;
  }

  checkAllRequestedDataTypes();

  // If all dependencies are resolved start all threads
  for (auto& sharedObject : loadedSharedObjects_)
  {
    sharedObject.start();
  }
}

void SharedObjectManager::checkAllRequestedDataTypes()
{
  // Compose a set of all requested datatypes
  std::unordered_set<std::type_index> unresolvedDependencies;
  for (auto& threadDatum : threadData_)
  {
    for (auto& sender : threadDatum.senders)
    {
      auto& requests = sender->getRequested();
      unresolvedDependencies.insert(requests.begin(), requests.end());
    }
  }

  for (auto& threadDatum : threadData_)
  {
    for (auto& receiver : threadDatum.receivers)
    {
      auto& produced = receiver->getProduced();
      for (auto& production : produced)
      {
        unresolvedDependencies.erase(production);
      }
    }
  }

  if (!unresolvedDependencies.empty())
  {
    Log(LogLevel::ERROR) << "Unresolved dependencies:";
    for (auto& unresolvedDependency : unresolvedDependencies)
    {
      Log(LogLevel::ERROR) << unresolvedDependency.name();
    }
    throw std::runtime_error("Could not produce all DataTypes!");
  }
}

void SharedObjectManager::stop()
{
  for (auto& sharedObject : loadedSharedObjects_)
  {
    sharedObject.stop();
  }
  for (auto& sharedObject : loadedSharedObjects_)
  {
    sharedObject.join();
  }
  loadedSharedObjects_.clear();
  threadData_.clear();
  conChannels_.clear();
}
