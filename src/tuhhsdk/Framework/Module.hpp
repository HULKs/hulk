#pragma once

#include <string>
#include <typeindex>
#include <unordered_set>

#include "Hardware/RobotInterface.hpp"
#include "Modules/Configuration/Configuration.h"
#include "Modules/Debug/Debug.h"

#include "Database.hpp"

#define ModuleName static constexpr const char*

class ModuleManagerInterface;

class ModuleBase
{
public:
  /**
   * @brief ModuleBase constructs the ModuleBase
   * @param manager the ModuleManager of that constructed this module
   * @param name the name of the module class, used for the mount point
   */
  ModuleBase(const ModuleManagerInterface& manager, const std::string& name);
  /**
   * @brief ~ModuleBase virtual destructor for polymorphism
   */
  virtual ~ModuleBase() = default;
  /**
   * @brief runCycle is called everytime some action has to be taken
   */
  virtual void runCycle() = 0;
  /**
   * @brief getDependencies getter method for dependencies
   * @return the set of dependencies
   */
  const std::unordered_set<std::type_index>& getDependencies() const
  {
    return dependencies_;
  }
  /**
   * @brief getProductions getter method for productions
   * @return the set of productions
   */
  const std::unordered_set<std::type_index>& getProductions() const
  {
    return productions_;
  }

protected:
  /**
   * @brief debug provides access to the Debug instance
   * @return the Debug instance
   */
  DebugDatabase::DebugMap& debug() const
  {
    return *debug_;
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
  /// the mount point used for this module
  const std::string mount_;
  /// the name of the manager (ModuleManagerInterface)
  const std::string managerName_;

private:
  /// the ModuleManager that is responsible for this module
  const ModuleManagerInterface& manager_;
  /// the Database of the ModuleManager
  Database& database_;
  /// the Debug instance
  DebugDatabase::DebugMap*& debug_;
  /// the Configuration instance
  Configuration& configuration_;
  /// the RobotInterface instance
  RobotInterface& robotInterface_;
  /// the set of dependencies of this module
  std::unordered_set<std::type_index> dependencies_;
  /// the set of productions of this module
  std::unordered_set<std::type_index> productions_;
  template <typename T, typename T2>
  friend class Module;
  template <typename T>
  friend class Dependency;
  template <typename T>
  friend class Reference;
  template <typename T>
  friend class Production;
  template <typename T>
  friend class Parameter;
};

template <typename T>
class ModuleFactoryBase
{
public:
  /**
   * @brief ~ModuleFactoryBase virtual destructor for polymorphism
   */
  virtual ~ModuleFactoryBase() = default;
  /**
   * @brief produce creates an instance of a module
   * @param manager the module manager controlling the module
   * @return a shared pointer to the newly created module
   */
  virtual std::shared_ptr<ModuleBase> produce(const ModuleManagerInterface& manager) = 0;
  /**
   * @brief getter for the name of the module produced by this factory
   * @return the name of the corresponding module
   */
  virtual const char* getName() = 0;
  /// next pointer in the factory linked list
  ModuleFactoryBase* next;
  /// the first element of the linked list (the last constructed module). It must be defined in e.g.
  /// the module manager file.
  static ModuleFactoryBase* begin;
};

template <typename T, typename T2>
class ModuleFactory : public ModuleFactoryBase<T2>
{
public:
  /**
   * @brief ModuleFactory is called at static initialization time and registers the factory
   */
  ModuleFactory()
  {
    ModuleFactoryBase<T2>::next = ModuleFactoryBase<T2>::begin;
    ModuleFactoryBase<T2>::begin = this;
  }
  /**
   * @brief use does nothing but is needed
   */
  void use() {}
  /**
   * @brief produce creates an instance of a module
   * @param manager the module manager controlling the module
   * @return a shared pointer to the newly created module
   */
  std::shared_ptr<ModuleBase> produce(const ModuleManagerInterface& manager)
  {
    return std::make_shared<T>(manager);
  }
  /**
   * @brief getter for the name of the module produced by this factory
   * @return the name of the corresponding module
   */
  virtual const char* getName()
  {
    return T::name;
  }
};

template <typename T, typename T2>
class Module : public ModuleBase
{
public:
  /**
   * @brief Module constructs the module
   * @param manager the ModuleManager that created this module
   * @param name the name of the module
   */
  Module(const ModuleManagerInterface& manager)
    : ModuleBase(manager, T::name)
  {
    // This call is needed so that the compiler actually creates the class ModuleFactory<T>.
    factory.use();
  }
  /**
   * @brief runCycle
   */
  void runCycle() final
  {
    for (auto& pro : productions_)
    {
      database_.reset(pro);
    }
    cycle();
    for (auto& pro : productions_)
    {
      auto& realProduction = database_.get(pro);
      const std::string mount = managerName_ + "." + realProduction.getName();
      // only send if autoUpdate is enabled and the mount is subscribed.
      if (realProduction.autoUpdateDebug && debug().isSubscribed(mount))
      {
        debug().update(mount, realProduction);
      }
    }
  }
  /**
   * @brief cycle is called everytime some action has to be taken
   */
  virtual void cycle() = 0;
  /**
   * @brief ~Module virtual destructor for polymorphism
   */
  virtual ~Module() = default;

private:
  /// factory that is used to create an instance of T
  static ModuleFactory<T, T2> factory;
};

template <typename T, typename T2>
ModuleFactory<T, T2> Module<T, T2>::factory;

template <typename T>
class Dependency
{
public:
  /**
   * @brief Dependency obtains a reference to the datum from the database and registers in the
   * module
   * @brief module the module owning this Dependency
   */
  Dependency(ModuleBase& module)
    : data_(module.database_.get<T>())
  {
    module.dependencies_.emplace(typeid(T));
  }
  /**
   * @brief operator-> is used to access members of the dependency
   * @return a pointer to the datum in the database
   */
  const T* operator->() const
  {
    return &data_;
  }
  /**
   * @brief operator* can be used specifically for assignments
   * @return a reference to the datum in the database
   */
  const T& operator*() const
  {
    return data_;
  }

private:
  /// a reference to the datum in the database
  const T& data_;
};

template <typename T>
class Reference
{
public:
  /**
   * @brief Reference obtains a reference to the datum from the database
   * @brief module the module owning this reference
   */
  Reference(ModuleBase& module)
    : data_(module.database_.get<T>())
  {
  }
  /**
   * @brief operator-> is used to access members of the reference
   * @return a pointer to the datum in the database
   */
  const T* operator->() const
  {
    return &data_;
  }
  /**
   * @brief operator* can be used specifically for assignments
   * @return a reference to the datum in the database
   */
  const T& operator*() const
  {
    return data_;
  }

private:
  /// a reference to the datum in the database
  const T& data_;
};

template <typename T>
class Production
{
public:
  /**
   * @brief Production obtains a reference to the datum from the database and registers in the
   * module
   * @param module the module owning this Production
   * @param autoUpdateDebug whether this DataType should automatically being sent via debug (if
   * subscribed)
   */
  Production(ModuleBase& module, bool autoUpdateDebug = true)
    : data_(module.database_.get<T>())
  {
    module.productions_.emplace(typeid(T));
    data_.autoUpdateDebug = autoUpdateDebug;
  }
  /**
   * @brief operator-> is used to access members of the production
   * @return a pointer to the datum in the database
   */
  T* operator->()
  {
    return &data_;
  }
  /**
   * @brief operator-> a const version because the producing function may have const methods
   * accessing data
   * @return a pointer to the datum in the database
   */
  const T* operator->() const
  {
    return &data_;
  }
  /**
   * @brief operator* can be used specifically for assignments
   * @return a reference to the datum in the database
   */
  T& operator*()
  {
    return data_;
  }
  /**
   * @brief operator* a const version, e.g. for a right hand side operand in assignments
   * @return a reference to the datum in the database
   */
  const T& operator*() const
  {
    return data_;
  }

private:
  /// a reference to the datum in the database
  T& data_;
};

template <typename T>
class Parameter
{
public:
  /**
   * @brief Parameter gets the value of the parameter and may register a callback handler
   * @param module the module that uses this parameter
   * @param key the name of this parameter
   * @param callback is called everytime the value is changed
   */
  Parameter(const ModuleBase& module, const std::string& key,
            std::function<void()> callback = std::function<void()>())
    : value_()
    , callback_(callback)
  {
    module.configuration_.get(module.mount_, key) >> value_;
    if (callback_)
    {
      module.configuration_.registerCallback(module.mount_, key,
                                             boost::bind(&Parameter<T>::onUpdate, this, _1));
    }
  }
  /**
   * @brief operator() a non-const version because some program modification might be needed
   * @return a reference to the parameter
   */
  T& operator()()
  {
    return value_;
  }
  /**
   * @brief operator() returns a reference to the value of the parameter
   * @return a reference to the parameter
   */
  const T& operator()() const
  {
    return value_;
  }

private:
  /**
   * @brief onUpdate is called by the Configuration class whenever the value changes (e.g. over
   * network)
   * @param value the new value
   */
  void onUpdate(const Uni::Value& value)
  {
    value >> value_;
    callback_();
  }
  /// stores the actual value
  T value_;
  /// the callback for value changes
  std::function<void()> callback_;
};

template <typename T>
ModuleFactoryBase<T>* ModuleFactoryBase<T>::begin = nullptr;
