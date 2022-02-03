#pragma once

#include <string>

#ifdef ITTNOTIFY_FOUND
#include <ittnotify.h>
#endif

#include "Framework/Configuration/Configuration.h"
#include "Framework/Debug/Debug.h"
#include "Hardware/RobotInterface.hpp"

#include "Tools/Math/MovingAverage.hpp"

#include "Framework/Database.hpp"
#include "Framework/DebugDatabase.hpp"
#include "Framework/Module.hpp"

#include "Framework/Log/Log.hpp"


class ModuleManagerInterface
{
public:
  /**
   * @brief ModuleManagerInterface intializes the member variables
   * @param name the name of the module manager
   * @param configurationType the configuration type of the modules in this manager
   * @param senders the list of senders for this module manager
   * @param receivers the list of receivers for this module manager
   * @param debug the Debug instance
   * @param configuration the Configuration instance
   * @param robotInterface the RobotInterface instance
   */
  ModuleManagerInterface(const std::string& name, ConfigurationType configurationType,
                         const std::vector<Sender*>& senders,
                         const std::vector<Receiver*>& receivers, Debug& debug,
                         Configuration& configuration, RobotInterface& robotInterface);
  /**
   * @brief ~ModuleManagerInterface virtual destructor for polymorphism
   */
  virtual ~ModuleManagerInterface();
  /**
   * @brief getDatabase returns a reference to the database for this ModuleManager
   * @return the database for this ModuleManager
   */
  Database& getDatabase() const;
  /**
   * @brief getName returns a name that is identifying the ModuleManager
   * @return the name of the ModuleManager
   */
  const std::string& getName() const;
  /**
   * @brief getConfigurationType returns whether the modules of this ModuleManager are head or body
   * related
   * @return either ConfigurationType::HEAD or ConfigurationType::BODY
   */
  ConfigurationType getConfigurationType() const;
  /**
   * @brief debug provides access to the Debug instance
   * @return the Debug instance
   */
  DebugDatabase::DebugMap*& debug() const;
  /**
   * @brief getDebugDatabases returns the debug databases
   * @return a vector of debug databases
   */
  std::vector<const DebugDatabase*> getDebugDatabases() const;
  /**
   * @brief configuration provides access to the Configuration instance
   * @return the Configuration instance
   */
  Configuration& configuration() const
  {
    return configuration_;
  }
  /**
   * @brief robotInterface provides access to the RobotInterface instance
   * @return the RobotInterface instance
   */
  RobotInterface& robotInterface() const
  {
    return robotInterface_;
  }
  /**
   * @brief runCycle should be called at the beginning of each cycle
   */
  void runCycle();
  /**
   * @brief cycle calls all the modules of this ModuleManager
   */
  virtual void cycle() = 0;

protected:
  /**
   * @brief sortModules sorts the modules to a runnable order
   * @param T the type of the module manager
   * @return true iff the sorting was successful
   */
  template <typename T>
  bool sortModules();
  /// list of all modules in this module manager
#ifdef ITTNOTIFY_FOUND
  std::list<std::pair<std::shared_ptr<ModuleBase>, __itt_string_handle*>> modules_;
#else
  std::list<std::shared_ptr<ModuleBase>> modules_;
#endif


private:
  /// a name identifying the module manager
  const std::string name_;
  /// the default configuration type of the modules in this manager
  const ConfigurationType configurationType_;
  /// a central storage for all data types that are moved between modules
  Database database_;
  /// the Debug database
  DebugDatabase debugDatabase_;
  /// the current debug map
  DebugDatabase::DebugMap* currentDebugMap_ = nullptr;
  /// the Debug instance
  Debug& debug_;
  /// the Configuration instance
  Configuration& configuration_;
  /// the RobotInterface instance
  RobotInterface& robotInterface_;
  /// the time the cycle needed to be executed. Averaged over 60 cycles.
  SimpleArrayMovingAverage<std::chrono::duration<float, std::chrono::seconds::period>,
                           std::chrono::duration<float, std::chrono::seconds::period>, 60>
      averageCycleTime_;
};

template <typename T>
// NOLINTNEXTLINE(readability-function-cognitive-complexity)
bool ModuleManagerInterface::sortModules()
{
// a list of all modules that should be constructed (not yet sorted by execution order)
#ifdef ITTNOTIFY_FOUND
  std::list<std::pair<std::shared_ptr<ModuleBase>, __itt_string_handle*>> unsortedModules;
#else
  std::list<std::shared_ptr<ModuleBase>> unsortedModules;
#endif
  // create instances of all module types in this module manager
  for (ModuleFactoryBase<T>* factory = ModuleFactoryBase<T>::begin__; factory != nullptr;
       factory = factory->next)
  {
    // check if corresponding key exists in the activeModulesMap
    if (configuration_.hasProperty("tuhhSDK.moduleSetup", factory->getName()))
    {
      // check if the module should be constructed
      if (configuration_.get("tuhhSDK.moduleSetup", factory->getName()).asBool())
      {
        // create instaces of the modules that are in the list of active modules
#ifdef ITTNOTIFY_FOUND
        unsortedModules.push_back(std::make_pair(factory->produce(*this),
                                                 __itt_string_handle_create(factory->getName())));
#else
        unsortedModules.push_back(factory->produce(*this));
#endif
      }
    }
    else
    {
      throw std::runtime_error(std::string("Module ") + factory->getName() +
                               " not found in activeModuleMap. Have you forgotten to add it?");
    }
  }

  std::unordered_set<std::type_index> allDependencies;
  std::unordered_set<std::type_index> allProductions;
  std::unordered_set<std::type_index> productions;

  // find all datatypes that are produced or depended on in this module manager
  for (auto& module : unsortedModules)
  {
#ifdef ITTNOTIFY_FOUND
    auto& moduleBase = module.first;
#else
    auto& moduleBase = module;
#endif

    allDependencies.insert(moduleBase->getDependencies().begin(),
                           moduleBase->getDependencies().end());
    // Insert all productions into allProductions and perform sanity checks on the module setup
    for (auto production : moduleBase->getProductions())
    {
      if (auto productionSearch = allProductions.find(production);
          productionSearch != allProductions.end())
      {
        Log<M_TUHHSDK>(LogLevel::ERROR) << "Production with type id " << productionSearch->name()
                                        << " is produced multiple times.";
        throw std::runtime_error("Module setup sanity check failed");
      }
      allProductions.emplace(production);
    }
  }
  // all datatypes that are not produced in this module manager but are depended on are requested
  // from other module managers
  for (auto& dependency : allDependencies)
  {
    if (allProductions.find(dependency) == allProductions.end())
    {
      // We check inside the SharedObjectManager if all dependencies are fulfilled
      database_.request(dependency);
      productions.emplace(dependency);
    }
  }

  // topological sorting
  unsigned int size;
  do
  {
    size = unsortedModules.size();
    for (auto it = unsortedModules.begin(); it != unsortedModules.end();)
    {
#ifdef ITTNOTIFY_FOUND
      auto& moduleBase = it->first;
#else
      auto& moduleBase = *it;
#endif
      bool add = true;
      for (auto itDependency : moduleBase->getDependencies())
      {
        if (!productions.count(itDependency))
        {
          add = false;
          break;
        }
      }
      if (add)
      {
        productions.insert(moduleBase->getProductions().begin(),
                           moduleBase->getProductions().end());
        modules_.push_back(*it);
        it = unsortedModules.erase(it);
      }
      else
      {
        it++;
      }
    }
  } while (unsortedModules.size() < size);

  for (auto& mod : modules_)
  {
#ifdef ITTNOTIFY_FOUND
    auto& moduleBase = mod.first;
#else
    auto& moduleBase = mod;
#endif
    for (auto& production : moduleBase->getProductions())
    {
      getDatabase().produce(production);
    }
  }

  return unsortedModules.empty();
}
