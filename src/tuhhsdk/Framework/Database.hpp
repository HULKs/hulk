#pragma once

#include <typeindex>
#include <unordered_map>

#include "DataType.hpp"
#include "Messaging.hpp"


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
    auto it = data_map_.find(type);
    if (it == data_map_.end())
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
    auto it = data_map_.find(type);
    if (it == data_map_.end())
    {
      T* new_object = new T;
      new_object->reset();
      DatabaseEntry entry(new_object);
      data_map_.emplace(type, entry);
      return *new_object;
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
    DatabaseEntry(DataTypeBase* const data)
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
  std::unordered_map<std::type_index, DatabaseEntry> data_map_;
  /// list of registered senders
  std::vector<Sender*> senders_;
  /// list of registered receivers
  std::vector<Receiver*> receivers_;
};
