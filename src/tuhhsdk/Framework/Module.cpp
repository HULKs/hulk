#include "Modules/Configuration/Configuration.h"

#include "Data/ReplayData.hpp"
#include "Module.hpp"
#include "ModuleManagerInterface.hpp"
#include "print.h"

ModuleBase::ModuleBase(const ModuleManagerInterface& manager, const std::string& name)
  : mount_(manager.getName() + "." + name)
  , managerName_(manager.getName())
  , manager_(manager)
  , database_(manager_.getDatabase())
  , debug_(manager_.debug())
  , configuration_(manager_.configuration())
  , robotInterface_(manager_.robotInterface())
{
  if (!configuration_.mount(mount_, name + ".json", manager_.getConfigurationType()))
  {
    return;
  }

  ReplayConfigurations replayConfig;
  if (robotInterface_.getFakeData().getFakeData(replayConfig))
  {
    // Try to set the configuration values
    for (auto& c : replayConfig.data)
    {
      // Check if the mount is the correct one
      if (c.mount != mount_)
      {
        continue;
      }
      bool isBlacklisted = false;
      for (auto it = configuration_.get("tuhhSDK.base", "replayConfigMountBlacklist").objectBegin();
           it != configuration_.get("tuhhSDK.base", "replayConfigMountBlacklist").objectEnd() &&
           !isBlacklisted;
           it++)
      {
        if (it->first == c.mount)
        {
          for (auto keyIt = it->second.vectorBegin(); keyIt != it->second.vectorEnd(); keyIt++)
          {
            if (keyIt->asString() == "*" || keyIt->asString() == c.key)
            {
              isBlacklisted = true;
              break;
            }
          }
        }
      }
      if (isBlacklisted)
      {
        Log(LogLevel::INFO) << "Skipping replay configuration mount " << c.mount << " Key "
                            << c.key;
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
