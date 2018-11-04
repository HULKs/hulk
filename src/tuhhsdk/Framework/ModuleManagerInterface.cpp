#include "Module.hpp"
#include "ModuleManagerInterface.hpp"


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
  cycle();
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

const std::string& ModuleManagerInterface::getName() const
{
  return name_;
}

ConfigurationType ModuleManagerInterface::getConfigurationType() const
{
  return configurationType_;
}
