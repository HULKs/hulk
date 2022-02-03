#pragma once

#include "Hardware/Clock.hpp"
#include "Tools/Storage/Image.hpp"
#include "Tools/Storage/UniValue/UniValue.h"
#include "Tools/Var/SpscQueue.hpp"

#include <array>
#include <atomic>
#include <string>
#include <unordered_map>

/**
 * All sounds that can be played
 */
enum class AudioSounds
{
  OUCH = 0,
  BALL = 1,
  DONK = 2,
  LEFT = 3,
  RIGHT = 4,
  FRONT = 5,
  FRONT_LEFT = 6,
  FRONT_RIGHT = 7,
  REAR = 8,
  REAR_LEFT = 9,
  REAR_RIGHT = 10,
  CAMERA_RESET = 11,
  CENTER_CIRCLE = 12,
  LOLA_DESYNC = 13,
  PENALTY_AREA = 14,
  PENALTY_SPOT = 15,
  SQUAT = 16,
  T_JUNCTION = 17,
  PLAYING_ROLE_STRIKER = 18,
  PLAYING_ROLE_KEEPER = 19,
  PLAYING_ROLE_REPLACEMENT_KEEPER = 20,
  PLAYING_ROLE_SUPPORTER = 21,
  PLAYING_ROLE_BISHOP = 22,
  PLAYING_ROLE_DEFENDER = 23,
  PLAYING_ROLE_DEFENDER_LEFT = 24,
  PLAYING_ROLE_DEFENDER_RIGHT = 25,
  FALSE_POSITIVE_DETECTED = 26,
  FALSE_POSITIVE = 27,
  WEEEEE = 28,
  DRIFT = 29,
  SAME_PLAYER_NUMBER_MIN = 100,
  SAME_PLAYER_NUMBER_21 = 101,
  SAME_PLAYER_NUMBER_22 = 102,
  SAME_PLAYER_NUMBER_23 = 103,
  SAME_PLAYER_NUMBER_24 = 104,
  SAME_PLAYER_NUMBER_25 = 105,
  SAME_PLAYER_NUMBER_26 = 106,
  SAME_PLAYER_NUMBER_27 = 107,
  SAME_PLAYER_NUMBER_28 = 108,
  SAME_PLAYER_NUMBER_29 = 109,
  SAME_PLAYER_NUMBER_30 = 110,
  SAME_PLAYER_NUMBER_31 = 111,
  SAME_PLAYER_NUMBER_32 = 112,
  SAME_PLAYER_NUMBER_33 = 113,
  SAME_PLAYER_NUMBER_34 = 114,
  SAME_PLAYER_NUMBER_35 = 115,
  SAME_PLAYER_NUMBER_36 = 116,
  SAME_PLAYER_NUMBER_MAX = 117,
  SAME_PLAYER_NUMBER_GENERAL_ETH = 118,
  SAME_PLAYER_NUMBER_GENERAL_WIFI = 119,
  USB_STICK_MISSING = 120
};

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
    for (auto& debugMap : debugMaps_)
    {
      debugMap.debugDatabase_ = this;
    }
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
    Clock::time_point updateTime;
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
     * @brief playAudio sends an audio sound requests
     * @param key The key of the audio playing
     * @param audioSound The sound to play
     */
    void playAudio(const std::string& key, AudioSounds audioSound);
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
    void setUpdateTime(Clock::time_point updateTime);
    /**
     * @brief getUpdateTime returns the update time of the current debugMap
     * @return the updateTime
     */
    Clock::time_point getUpdateTime() const;
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
    Clock::time_point updateTime_;
    /// Pointer to the debug database
    DebugDatabase* debugDatabase_ = nullptr;

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
  /**
   * @brief popLastRequestedSound returns the last requested sound that was requested.
   * @param audioSound a reference to the sound
   * @return if there was a requested sound
   */
  bool popLastRequestedSound(AudioSounds& audioSound) const;


private:
  /// The debug maps. Tripple buffer for sake of thread safeness
  std::array<DebugMap, 3> debugMaps_;
  /// the map index to the map that is currently used for updates
  std::size_t currentMapIndex_;
  /// the map that is currently being transported by a 'transporter' (DebugTransportInterface)
  DebugMap* currentlyTransportedMap_;
  /// the timepoint when the currentlyTransportedMap was finalized (cycle finished) by a debug
  /// source.
  Clock::time_point currentTransportMapUpdateTime_;
  /// the last finished DebugMap
  std::atomic<DebugMap*> nextDebugMapToTransport_;
  /// audio log requests
  mutable SpscRing<AudioSounds, 20> requestedSounds_;

  friend class DebugMap;
};
