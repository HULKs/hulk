#include "Modules/Configuration/Configuration.h"

#include "Module.hpp"
#include "ModuleManagerInterface.hpp"


ModuleBase::ModuleBase(const ModuleManagerInterface& manager, const std::string& name)
  : mount_(manager.getName() + "." + name)
  , manager_(manager)
  , database_(manager_.getDatabase())
  , debug_(manager_.debug())
  , configuration_(manager_.configuration())
  , robotInterface_(manager_.robotInterface())
{
  try
  {
    configuration_.mount(mount_, name + ".json", manager_.getConfigurationType());
  }
  catch (ConfigurationException& e)
  {
    if (e.getErrorType() != ConfigurationException::FILE_NOT_FOUND)
    {
      throw;
    }
  }
}
