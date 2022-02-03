#pragma once

#include <typeindex>
#include <unordered_map>

#include "Framework/DataType.hpp"
#include "Framework/Messaging.hpp"

#include "Framework/Log/Log.hpp"


class Database
{
public:
  /**
   * @brief ~Database deletes all stored data
   */
  ~Database();
  /**
   * @brief reset resets one data type with a given type
   * @param type the type that shall be reset
   */
  void reset(const std::type_index& type);
  /**
   * @brief send sends all requested data types via the matching senders
   */
  void send();
  /**
   * @brief receive receives all incoming data types from receivers
   */
  void receive();
  /**
   * @brief request notifies senders that this Database wants to have a copy of a DataType
   * @param type the type that is requested
   */
  void request(const std::type_index& type);
  /**
   * @brief produce tells all other managers that we produce the this datatype
   * @param type the type we are going to produce
   */
  void produce(const std::type_index& type);
  /**
   * @brief addSender adds a sender to the database
   * @param sender a sender that will be added to the database
   */
  void addSender(Sender* sender);
  /**
   * @brief addReceiver adds a receiver to the database
   * @param receiver a receiver that will be added to the database
   */
  void addReceiver(Receiver* receiver);
  /**
   * @brief get obtains a reference to the DataType for the given type_index
   * @param type the type_index
   * @return a reference to the DataType
   */
  DataTypeBase& get(const std::type_index& type)
  {
    auto it = dataMap_.find(type);
    if (it == dataMap_.end())
    {
      throw std::runtime_error("Could not find DataType, but should be present here.");
    }
    return *(it->second.data);
  }
  /**
   * @brief get obtains the datum of a specific data type
   * @return a reference to the object inside the database
   */
  template <typename T>
  T& get()
  {
    const std::type_index& type = typeid(T);
    auto it = dataMap_.find(type);
    // create a new DataBaseEntry in case it did not exist.
    if (it == dataMap_.end())
    {
      T* newObject = new T;
      newObject->reset();
      DatabaseEntry entry(newObject);
      // Sanity check: Check for name duplicates in LOCAL database
      if (auto foundValue = std::find_if(dataMap_.begin(), dataMap_.end(),
                                         [&entry](const auto& otherValue) {
                                           return otherValue.second.data->getName() ==
                                                  entry.data->getName();
                                         });
          foundValue != dataMap_.end())
      {
        Log<M_TUHHSDK>(LogLevel::ERROR)
            << "Database: There are multiple DataTypes with the same name. Type index "
            << type.name() << " and " << foundValue->first.name() << " share the name "
            << entry.data->getName();
        throw std::runtime_error("Fatal Database error");
      }
      dataMap_.emplace(type, entry);
      return *newObject;
    }
    return *dynamic_cast<T*>(it->second.data);
  }

private:
  struct DatabaseEntry
  {
    /**
     * @brief DatabaseEntry creates an (unimported) database entry
     * @param data the pointer to the actual data
     */
    explicit DatabaseEntry(DataTypeBase* const data)
      : data(data)
      , imported(false)
    {
    }
    /// pointer to the actual datum
    DataTypeBase* const data;
    /// whether this datum is imported from another database
    bool imported;
  };
  /// This map stores exactly one instance per data type.
  std::unordered_map<std::type_index, DatabaseEntry> dataMap_;
  /// list of registered senders
  std::vector<Sender*> senders_;
  /// list of registered receivers
  std::vector<Receiver*> receivers_;
};
