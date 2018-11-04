#include "Modules/Configuration/Configuration.h"

#include "Data/ReplayData.hpp"
#include "Module.hpp"
#include "ModuleManagerInterface.hpp"

ModuleBase::ModuleBase(const ModuleManagerInterface& manager, const std::string& name)
  : mount_(manager.getName() + "." + name)
  , managerName_(manager.getName())
  , manager_(manager)
  , database_(manager_.getDatabase())
  , debug_(manager_.debug())
  , configuration_(manager_.configuration())
  , robotInterface_(manager_.robotInterface())
{
  try
  {
    configuration_.mount(mount_, name + ".json", manager_.getConfigurationType());
    ReplayConfigurations replayConfig;
    if (robotInterface_.getFakeData().getFakeData(replayConfig))
    {
      // Try to set the configuration values
      auto mounts = configuration_.getMountPoints();
      for (auto& c : replayConfig.data)
      {
        // Check if the mount is the correct one
        if (c.mount != mount_)
        {
          continue;
        }
        // Only set the key when it still exists
        if (!configuration().hasProperty(c.mount, c.key))
        {
          continue;
        }
        configuration().set(c.mount, c.key, c.data);
      }
    }
  }
  catch (ConfigurationException& e)
  {
    if (e.getErrorType() != ConfigurationException::FILE_NOT_FOUND)
    {
      throw;
    }
  }
}
