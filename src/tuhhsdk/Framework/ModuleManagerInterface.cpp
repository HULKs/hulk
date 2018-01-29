#include "ModuleManagerInterface.hpp"
#include "Module.hpp"


ModuleManagerInterface::ModuleManagerInterface(
  const std::string& name,
  const ConfigurationType configurationType,
  const std::vector<Sender*>& senders,
  const std::vector<Receiver*>& receivers,
  Debug& debug,
  Configuration& configuration,
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
}

Database& ModuleManagerInterface::getDatabase() const
{
  // Sorry for the const_cast.
  return const_cast<Database&>(database_);
}

const std::string& ModuleManagerInterface::getName() const
{
  return name_;
}

ConfigurationType ModuleManagerInterface::getConfigurationType() const
{
  return configurationType_;
}
