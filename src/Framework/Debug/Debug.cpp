#include "Framework/Debug/Debug.h"
#include "Framework/Debug/Subscription.h"


Debug::Debug()
{
#ifdef ITTNOTIFY_FOUND
  debugDomain_ = __itt_domain_create("Debug");
  transportString_ = __itt_string_handle_create("transporting");
#endif
}

void Debug::subscribe(const std::string& key)
{
  bool subscriptionSuccessful = false;
  for (auto& debugSourceIt : debugSources_)
  {
    if (debugSourceIt.second.debugDatabase->subscribe(key))
    {
      subscriptionSuccessful = true;
      break;
    }
  }

  if (!subscriptionSuccessful)
  {
    auto entry = outstandingSubscriptions_.find(key);
    if (entry == outstandingSubscriptions_.end())
    {
      entry = outstandingSubscriptions_.insert(std::make_pair(key, 0)).first;
    }
    entry->second.fetch_add(1);
  }
}

void Debug::unsubscribe(const std::string& key)
{
  auto entry = outstandingSubscriptions_.find(key);
  if (entry != outstandingSubscriptions_.end())
  {
    entry->second.fetch_sub(1);
  }

  for (auto& debugSourceIt : debugSources_)
  {
    debugSourceIt.second.debugDatabase->unsubscribe(key);
  }
}

void Debug::resolveOutstandingSubscriptions()
{
  if (outstandingSubscriptions_.empty())
  {
    return;
  }

  for (auto subscription = outstandingSubscriptions_.begin();
       subscription != outstandingSubscriptions_.end();)
  {
    bool subscriptionSuccessful = false;
    for (auto& debugSourceIt : debugSources_)
    {
      if (debugSourceIt.second.debugDatabase->subscribe(subscription->first))
      {
        subscription->second.fetch_sub(1);
        subscriptionSuccessful = true;
        break;
      }
    }

    if (subscriptionSuccessful && subscription->second.load() == 0)
    {
      subscription = outstandingSubscriptions_.erase(subscription);
    }
    else
    {
      ++subscription;
    }
  }
}

void Debug::addTransport(const std::shared_ptr<DebugTransportInterface>& transport)
{
  transporter_.push_back(transport);
}

void Debug::removeAllTransports()
{
  transporter_.clear();
}

void Debug::addDebugSource(const std::string& debugSourceName, DebugDatabase* debugDatabase)
{
  debugSources_.emplace(std::make_pair(debugSourceName, DebugSource(debugDatabase)));
}

void Debug::removeDebugSource(const std::string& debugSourceName)
{
  stop();
  debugSources_.erase(debugSourceName);
}

const std::unordered_map<std::string, Debug::DebugSource>& Debug::getDebugSources() const
{
  return debugSources_;
}

void Debug::start()
{
  if (transporter_.empty())
  {
    return;
  }
  shutdownThread_ = false;
  transporterThread_ = std::thread([this]() { run(); });
}

void Debug::stop()
{
  if (transporterThread_.joinable())
  {
    shutdownThread_ = true;
    trigger();
    transporterThread_.join();
  }
}

void Debug::run()
{
#ifdef ITTNOTIFY_FOUND
  __itt_thread_set_name("Debug");
#endif

  while (true)
  {
    {
      std::unique_lock<std::mutex> l2(transporterMutex_);
      transporterCondition_.wait(l2, [this] { return trigger_; });
      if (shutdownThread_)
      {
        break;
      }
      trigger_ = false;
    }
    resolveOutstandingSubscriptions();

    bool running = true;
    while (running)
    {
      running = false;
      for (auto& debugSource : debugSources_)
      {
        DebugDatabase::DebugMap* nextDebugMap =
            debugSource.second.debugDatabase->nextTransportableMap();
        if (nextDebugMap == nullptr || nextDebugMap == debugSource.second.currentDebugMap)
        {
          debugSource.second.currentDebugMap = nullptr;
          continue;
        }
        debugSource.second.currentDebugMap = nextDebugMap;
        running = true;
      }

      // Check whether we need to terminate for a shutdown
      if (shutdownThread_)
      {
        break;
      }

#ifdef ITTNOTIFY_FOUND
      __itt_task_begin(debugDomain_, __itt_null, __itt_null, transportString_);
#endif
      // Begin with the transporting
      for (const auto& transporter : transporter_)
      {
        transporter->transport();
      }
#ifdef ITTNOTIFY_FOUND
      __itt_task_end(debugDomain_);
#endif

      // Clean up the DebugMaps
      for (auto& debugSource : debugSources_)
      {
        debugSource.second.debugDatabase->finishTransporting();
      }
    }
    if (shutdownThread_)
    {
      break;
    }
  }
}

void Debug::trigger()
{
  {
    std::lock_guard<std::mutex> lg(transporterMutex_);
    trigger_ = true;
  }
  transporterCondition_.notify_one();
}
