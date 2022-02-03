#include "Framework/Configuration/Configuration.h"

#include "Data/ReplayData.hpp"
#include "Framework/Log/Log.hpp"
#include "Framework/Module.hpp"
#include "Framework/ModuleManagerInterface.hpp"

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
    for (auto& config : replayConfig.data)
    {
      // Check if the mount is the correct one
      if (config.mount != mount_)
      {
        continue;
      }
      bool isBlacklisted = false;
      for (auto it = configuration_.get("tuhhSDK.base", "replayConfigMountBlacklist").objectBegin();
           it != configuration_.get("tuhhSDK.base", "replayConfigMountBlacklist").objectEnd() &&
           !isBlacklisted;
           it++)
      {
        if (it->first == config.mount)
        {
          for (auto keyIt = it->second.vectorBegin(); keyIt != it->second.vectorEnd(); keyIt++)
          {
            if (keyIt->asString() == "*" || keyIt->asString() == config.key)
            {
              isBlacklisted = true;
              break;
            }
          }
        }
      }
      if (isBlacklisted)
      {
        Log<M_TUHHSDK>(LogLevel::INFO)
            << "Skipping replay configuration mount " << config.mount << " Key " << config.key;
        continue;
      }
      // Only set the key when it still exists
      if (!configuration().hasProperty(config.mount, config.key))
      {
        continue;
      }
      configuration().set(config.mount, config.key, config.data);
    }
  }
}
