#include "Tools/Storage/Image.hpp"

#include "DebugDatabase.hpp"

void DebugDatabase::DebugMap::sendImage(const std::string& key, const Image& image)
{
  assert(inUse_.load() == true &&
         "Map to update was not in use (at least we should mark this map as used)");
  // try to find the given key in the map
  auto it = debugMap_.find(key);
  if (it == debugMap_.end())
  {
    // Key was not found in map
    it = debugMap_
             .emplace(std::piecewise_construct, std::forward_as_tuple(key), std::forward_as_tuple())
             .first;
  }
  else if (it->second.subscribedCount.load() == 0)
  {
    // Key was found but is not subscribed. Return to reduce overhead
    return;
  }

  // Update the given key value pair.
  *(it->second.image) = image;
  it->second.updateTime = updateTime_;
  it->second.isImage = true;
}

void DebugDatabase::DebugMap::playAudio(const std::string& key, const AudioSounds aSound)
{
  assert(inUse_.load() == true &&
         "Map to update was not in use (at least we should mark this map as used)");

  // Inform the DebugDatabase about the requested sound
  debugDatabase_->requestedSounds_.push(aSound);

  // try to find the given key in the map
  auto it = debugMap_.find(key);
  if (it == debugMap_.end())
  {
    // Key was not found in map
    it = debugMap_
             .emplace(std::piecewise_construct, std::forward_as_tuple(key), std::forward_as_tuple())
             .first;
  }
  else if (it->second.subscribedCount.load() == 0)
  {
    // Key was found but is not subscribed. Return to reduce overhead
    return;
  }

  // Update the given key value pair.
  *(it->second.data) = Uni::Value(static_cast<int>(aSound));
  it->second.updateTime = updateTime_;
  it->second.isImage = false;
}

bool DebugDatabase::DebugMap::isSubscribed(const std::string& key)
{
  const auto debugEntry = debugMap_.find(key);
  if (debugEntry == debugMap_.end())
  {
    return true;
  }
  return debugEntry->second.subscribedCount.load() > 0;
}

void DebugDatabase::DebugMap::setUpdateTime(TimePoint updateTime)
{
  updateTime_ = updateTime;
}

TimePoint DebugDatabase::DebugMap::getUpdateTime() const
{
  return updateTime_;
}

bool DebugDatabase::subscribe(const std::string& key)
{
  bool foundKey = false;
  for (const auto& map : debugMaps_)
  {
    if (map.debugMap_.find(key) != map.debugMap_.end())
    {
      foundKey = true;
      break;
    }
  }

  if (foundKey)
  {
    for (auto& map : debugMaps_)
    {
      map.debugMap_[key].subscribedCount.fetch_add(1);
    }
  }

  return foundKey;
}

bool DebugDatabase::unsubscribe(const std::string& key)
{
  bool foundKey = false;
  for (const auto& map : debugMaps_)
  {
    if (map.debugMap_.find(key) != map.debugMap_.end())
    {
      foundKey = true;
      break;
    }
  }

  if (foundKey)
  {
    for (auto& map : debugMaps_)
    {
      map.debugMap_[key].subscribedCount.fetch_sub(1);
    }
  }

  return foundKey;
}

DebugDatabase::DebugMap* DebugDatabase::nextUpdateableMap()
{
  bool expected = false;
  DebugMap* debugMap = nullptr;
  do
  {
    expected = false;
    currentMapIndex_ = (currentMapIndex_ + 1) % 3;
    debugMap = &debugMaps_[currentMapIndex_];
  } while (!debugMap->inUse_.compare_exchange_weak(expected, true));

  assert(debugMap->inUse_.load() && "Something is fishy...");

  return debugMap;
}

void DebugDatabase::finishUpdating()
{
  DebugMap& debugMap = debugMaps_[currentMapIndex_];
  assert(debugMap.inUse_.load() && "You are trying to unlock a non locked debugMap");
  debugMap.inUse_.store(false);

  nextDebugMapToTransport_.store(&debugMap);
}

DebugDatabase::DebugMap* DebugDatabase::nextTransportableMap()
{
  bool expected = false;
  DebugMap* transportMap = nextDebugMapToTransport_.load();
  currentlyTransportedMap_ = transportMap;
  if (transportMap == nullptr)
  {
    return currentlyTransportedMap_;
  }

  if (!transportMap->inUse_.compare_exchange_weak(expected, true))
  {
    // failed to lock the new map.
    currentlyTransportedMap_ = nullptr;
  }
  else if (!(transportMap->updateTime_ > currentTransportMapUpdateTime_))
  {
    // new map is older than the currently transported map. Skip.
    transportMap->inUse_.store(false);
    currentlyTransportedMap_ = nullptr;
  }

  return currentlyTransportedMap_;
}

void DebugDatabase::finishTransporting()
{
  if (currentlyTransportedMap_ == nullptr)
  {
    return;
  }

  assert(currentlyTransportedMap_->inUse_.load() &&
         "You are trying to unlock a non locked debugMap");
  currentTransportMapUpdateTime_ = currentlyTransportedMap_->updateTime_;
  currentlyTransportedMap_->inUse_.store(false);
}

bool DebugDatabase::popLastRequestedSound(AudioSounds& aSound) const
{
  return requestedSounds_.pop(aSound);
}
