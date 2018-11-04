#pragma once

#include "Tools/Storage/Image.hpp"
#include "Tools/Storage/UniValue/UniValue.h"
#include "Tools/Time.hpp"

#include <array>
#include <atomic>
#include <string>
#include <unordered_map>

/**
 * @brief DebugDatabase stores a set of debug maps
 * The DebugDatabase manages the triple buffering and thread save access of the debug map(s)
 */
class DebugDatabase
{
public:
  /**
   * @brief DebugDatabase initializes members
   */
  DebugDatabase()
    : currentMapIndex_(0)
    , currentlyTransportedMap_(nullptr)
    , nextDebugMapToTransport_(nullptr)
  {
  }
  /// DebugDataBase copy constructor is deleted.
  DebugDatabase(const DebugDatabase&) = delete;
  /**
   * @brief DebugMapEntry stores values for the debug map.
   */
  struct DebugMapEntry
  {
    /**
     * @brief DebugMapEntry initializes members.
     */
    DebugMapEntry()
      : data(std::make_unique<Uni::Value>())
      , image(std::make_unique<Image>())
      , subscribedCount(0)
      , isImage(false)
    {
    }
    /// A pointer to a uni value
    std::unique_ptr<Uni::Value> data;
    /// A pointer to an image
    std::unique_ptr<Image> image;

    /// How often this data entry is subscribed at the moment
    std::atomic<unsigned int> subscribedCount;
    /// If this data entry is storing an image
    bool isImage;
    /// The last time this key was updated
    TimePoint updateTime;
  };

  /**
   * @brief DebugMap stores key-value pairs combined with a timestamp
   */
  class DebugMap
  {
  public:
    /**
     * @brief update writes the given value for the given key into the debug map
     * @tparam T the typename of the given value
     * @param key the key to update
     * @param value the value to update.
     */
    template <typename T>
    void update(const std::string& key, const T& value)
    {
      assert(inUse_.load() == true &&
             "Map to update was not in use (at least we should mark this map as used)");
      // try to find the given key in the map
      auto it = debugMap_.find(key);
      if (it == debugMap_.end())
      {
        // Key was not found in map
        // This is using the c++ piecewise_construct functionallity such that
        // the constructors from string and DebugMapEntry are called with the arguments from the
        // tuples
        it = debugMap_.emplace(std::piecewise_construct, std::forward_as_tuple(key), std::tuple<>())
                 .first;
      }
      else if (it->second.subscribedCount.load() == 0)
      {
        // Key was found but is not subscribed. Return to reduce overhead
        return;
      }
      // Update the given key value pair.
      it->second.updateTime = updateTime_;
      *(it->second.data) << value;
    }
    /**
     * @brief sendImage writes a given image for the given key into the debug map (like update())
     * @param key The key to update
     * @param image The image to update
     */
    void sendImage(const std::string& key, const Image& image);
    /**
     * @brief isSubscribed checks if the given key is subscribed (at least once)
     * @param key the key to check for subscribtions
     * @return bool; true if the key is subscribed at least once
     */
    bool isSubscribed(const std::string& key);
    /**
     * @brief setUpdateTime sets the time at which the map was last updated
     * @param updateTime the time at which the map was last updated
     */
    void setUpdateTime(TimePoint updateTime);
    /**
     * @brief getUpdateTime returns the update time of the current debugMap
     * @return the updateTime
     */
    TimePoint getUpdateTime() const;
    /**
     * @brief getDebugMap returns a const reference to the underlying debug map.
     * @return a reference to the debug map
     */
    const auto& getDebugMap() const
    {
      return debugMap_;
    }

  private:
    /// the debug map (key value pair)
    std::unordered_map<std::string, DebugMapEntry> debugMap_;
    /// if the debug map is currently in use (transport, modulemanager)
    std::atomic<bool> inUse_{false};
    /// Time when this map was updated
    TimePoint updateTime_;

    friend class DebugDatabase;
  };

  /**
   * @brief subscribe subscribes the given key.
   * Multiple subscriptions are allowed. It is ensured that a key stayes subscribed until
   * unsubscribe is called as often as a key was subscribed.
   * @param key the key to subscribe
   * @return if the subscriptions was successful, i.e. if the key exists in this debugSource
   */
  bool subscribe(const std::string& key);
  /**
   * @brief unsubscribe unsubscribes the given key.
   * @param key The key to unsubscribe
   * @return if the unsubscriptions was successful, i.e. if the key exists in this debugSource
   */
  bool unsubscribe(const std::string& key);
  /**
   * @brief returns a pointer to the next map in the tripple buffer
   * @note it is not guaranteed that the returned map is unused. You need to check
   * @return DebugMap* the pointer to the next map
   */
  DebugMap* nextUpdateableMap();
  /**
   * @brief finishUpdating is used to mark the current map to be sendable by the
   * debug transport.
   */
  void finishUpdating();
  /**
   * @brief nextTransportableMap will return the next map that is ready to be transported.
   * @return the next map to transport. Will return a nullptr if there is no debugMap available.
   */
  DebugMap* nextTransportableMap();
  /**
   * @brief finishTransporting returns the map to the pool of updateable maps.
   */
  void finishTransporting();


private:
  /// The debug maps. Tripple buffer for sake of thread safeness
  std::array<DebugMap, 3> debugMaps_;
  /// the map index to the map that is currently used for updates
  std::size_t currentMapIndex_;
  /// the map that is currently being transported by a 'transporter' (DebugTransportInterface)
  DebugMap* currentlyTransportedMap_;
  /// the timepoint when the currentlyTransportedMap was finalized (cycle finished) by a debug
  /// source.
  TimePoint currentTransportMapUpdateTime_;
  /// the last finished DebugMap
  std::atomic<DebugMap*> nextDebugMapToTransport_;
};
