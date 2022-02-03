#include "Framework/ModuleManagerInterface.hpp"
#include "Framework/Module.hpp"
#include "Tools/Chronometer.hpp"


ModuleManagerInterface::ModuleManagerInterface(const std::string& name,
                                               const ConfigurationType configurationType,
                                               const std::vector<Sender*>& senders,
                                               const std::vector<Receiver*>& receivers,
                                               Debug& debug, Configuration& configuration,
                                               RobotInterface& robotInterface)
  : name_(name)
  , configurationType_(configurationType)
  , database_()
  , debug_(debug)
  , configuration_(configuration)
  , robotInterface_(robotInterface)
{
  for (auto sender : senders)
  {
    database_.addSender(sender);
  }
  for (auto receiver : receivers)
  {
    database_.addReceiver(receiver);
  }
  debug_.addDebugSource(name_, &debugDatabase_);
}

ModuleManagerInterface::~ModuleManagerInterface()
{
  // Ensure that modules are deconstructed before the database
  modules_.clear();
  debug_.removeDebugSource(name_);
}

void ModuleManagerInterface::runCycle()
{
  currentDebugMap_ = debugDatabase_.nextUpdateableMap();

  timespec ts{};
  clock_gettime(CLOCK_THREAD_CPUTIME_ID, &ts);
  const auto startTime{ts.tv_sec * 1000000000ULL + ts.tv_nsec};

  try
  {
    cycle();
  }
  catch (...)
  {
    debugDatabase_.finishUpdating();
    throw;
  }

  clock_gettime(CLOCK_THREAD_CPUTIME_ID, &ts);
  const auto endTime{ts.tv_sec * 1000000000ULL + ts.tv_nsec};

  averageCycleTime_.put(
      std::chrono::duration_cast<std::chrono::duration<float, std::chrono::seconds::period>>(
          std::chrono::nanoseconds{endTime} - std::chrono::nanoseconds{startTime}));
  currentDebugMap_->update(getName() + ".measuredCycleTime",
                           averageCycleTime_.getAverage().count());
  debugDatabase_.finishUpdating();
  debug_.trigger();
}

Database& ModuleManagerInterface::getDatabase() const
{
  // Sorry for the const_cast. | AH
  return const_cast<Database&>(database_);
}

DebugDatabase::DebugMap*& ModuleManagerInterface::debug() const
{
  // Sorry for the const_cast. | NR
  return const_cast<DebugDatabase::DebugMap*&>(currentDebugMap_);
}

std::vector<const DebugDatabase*> ModuleManagerInterface::getDebugDatabases() const
{
  std::vector<const DebugDatabase*> databases;
  auto debugSources = debug_.getDebugSources();
  databases.reserve(debugSources.size());

  for (auto& debugSource : debugSources)
  {
    databases.emplace_back(debugSource.second.debugDatabase);
  }

  return databases;
}

const std::string& ModuleManagerInterface::getName() const
{
  return name_;
}

ConfigurationType ModuleManagerInterface::getConfigurationType() const
{
  return configurationType_;
}
