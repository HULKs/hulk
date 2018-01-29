#pragma once

#include <string>

#include "Hardware/RobotInterface.hpp"
#include "Modules/Configuration/Configuration.h"
#include "Modules/Debug/Debug.h"

#include "Database.hpp"
#include "Module.hpp"


class ModuleManagerInterface {
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
  ModuleManagerInterface(
    const std::string& name,
    const ConfigurationType configurationType,
    const std::vector<Sender*>& senders,
    const std::vector<Receiver*>& receivers,
    Debug& debug,
    Configuration& configuration,
    RobotInterface& robotInterface);
  /**
   * @brief ~ModuleManagerInterface virtual destructor for polymorphism
   */
  virtual ~ModuleManagerInterface() = default;
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
   * @brief getConfigurationType returns whether the modules of this ModuleManager are head or body related
   * @return either ConfigurationType::HEAD or ConfigurationType::BODY
   */
  ConfigurationType getConfigurationType() const;
  /**
   * @brief debug provides access to the Debug instance
   * @return the Debug instance
   */
  Debug& debug() const
  {
    return debug_;
  }
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
   * @brief cycle calls all the modules of this ModuleManager
   */
  virtual void cycle() = 0;

protected:
  /**
   * @brief sortModules sorts the modules to a runnable order
   * @param mMT the type of the module manager
   * @return true iff the sorting was successful
   */
  template<typename T>
  bool sortModules();
  /// list of all modules in this module manager
  std::list<std::shared_ptr<ModuleBase>> modules_;

private:
  /// a name identifying the module manager
  const std::string name_;
  /// the default configuration type of the modules in this manager
  const ConfigurationType configurationType_;
  /// a central storage for all data types that are moved between modules
  Database database_;
  /// the Debug instance
  Debug& debug_;
  /// the Configuration instance
  Configuration& configuration_;
  /// the RobotInterface instance
  RobotInterface& robotInterface_;
};

template<typename T>
bool ModuleManagerInterface::sortModules()
{
  std::list<std::shared_ptr<ModuleBase>> unsorted_modules;
  // create instances of all module types in this module manager
  for (ModuleFactoryBase<T>* factory = ModuleFactoryBase<T>::begin; factory != nullptr; factory = factory->next) {
    unsorted_modules.push_back(factory->produce(*this));
  }

  std::unordered_set<std::type_index> allDependencies;
  std::unordered_set<std::type_index> allProductions;
  std::unordered_set<std::type_index> productions;

  // find all datatypes that are produced or depended on in this module manager
  for (auto& module : unsorted_modules) {
    allDependencies.insert(module->getDependencies().begin(), module->getDependencies().end());
    allProductions.insert(module->getProductions().begin(), module->getProductions().end());
  }
  // all datatypes that are not produced in this module manager but are depended on are requested from other module managers
  for (auto& dependency : allDependencies) {
    if (allProductions.find(dependency) == allProductions.end()) {
      database_.request(dependency);
      // TODO: find out whether any other module manager could provide the datatype.
      productions.emplace(dependency);
    }
  }

  // topological sorting
  unsigned int size;
  do {
    size = unsorted_modules.size();
    for (auto it = unsorted_modules.begin(); it != unsorted_modules.end();) {
      bool add = true;
      for (auto it_dependency : (*it)->getDependencies()) {
        if (!productions.count(it_dependency)) {
          add = false;
          break;
        }
      }
      if (add) {
        productions.insert((*it)->getProductions().begin(), (*it)->getProductions().end());
        modules_.push_back(*it);
        it = unsorted_modules.erase(it);
      } else {
        it++;
      }
    }
  } while (unsorted_modules.size() < size);

  return unsorted_modules.empty();
}
