#pragma once

#include <condition_variable>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <unordered_map>
#include <vector>

#include "Framework/DebugDatabase.hpp"
#include "Framework/Debug/DebugTransportInterface.h"

#ifdef ITTNOTIFY_FOUND
#include <ittnotify.h>
#endif


/**
 * This class is used as a middleman between the different debugMap sources and the debug
 * transports. DebugSources and DebugTransports can be registered inside the Debug class which
 * handles updates for the DebugSource DebugMaps to trigger all registered transports. It is also
 * used to pass the subscriptions from the transports to the corresponding DebugSources.
 */
class Debug
{
public:
  Debug();
  /**
   * @brief DebugSource represents a source that needs a debugMap for writing debugData to it
   */
  struct DebugSource
  {
    /**
     * @brief DebugSource initializes members
     * @param debugDatabase a pointer to the debugDatabase. Needed to get the debugMap
     */
    DebugSource(DebugDatabase* debugDatabase)
      : debugDatabase(debugDatabase){};

    /// a pointer to the debugDatabase to get the debugMap for update from.
    DebugDatabase* debugDatabase;
    /// a pointer to the current debugMap to update.
    DebugDatabase::DebugMap* currentDebugMap = nullptr;
  };

  /**
   * @brief addTransport adds a transporter to the list of transporters.
   * The given transporter will be notified when a debugSource finished a
   * @param transport the transport to add.
   */
  void addTransport(const std::shared_ptr<DebugTransportInterface>& transport);
  /**
   * @brief removeAllTransports removes all registered transports.
   */
  void removeAllTransports();
  /**
   * @brief addDebugSource is needed to add a DebugDatabase as source for subscribing debug data
   * @param debugSourceName the name of the debugSource.
   * @param debugDatabase the destination to subscribe and unsubscribe requests
   */
  void addDebugSource(const std::string& debugSourceName, DebugDatabase* debugDatabase);
  /**
   * @brief removeDebugSource used to remove the debug source registered under the debugSourceName
   * @param debugSourceName the name of the debug source to remove
   */
  void removeDebugSource(const std::string& debugSourceName);
  /**
   * @brief starts the debug thread.
   * will return with no debug thread running if the list of transporters is empty.
   */
  void start();
  /**
   * @brief sends a shutdown signal to this thread.
   * Will trigger all transporters one last time. Afterwards this thread is stopped.
   */
  void stop();
  /**
   * @brief the main function that is being executed in this thread.
   */
  void run();
  /**
   * @brief wakes this thread up.
   */
  void trigger();
  /**
   * @brief getDebugSources returns the debug sources
   * @return a const reference  to a map of debug sources.
   */
  const std::unordered_map<std::string, DebugSource>& getDebugSources() const;
  /**
   * @brief subscribe ensures that the given key will be transported in the next cycle
   * If the given key was not found, a dummy entry will be created (because it might be available in
   * the future)
   * If the given key was already subscribed, the subscribedCount will be increased to
   * ensure that the other subscriber cannot unsubscribe this key without the second one
   * unsubscripted.
   * @param key the key to subscribe
   */
  void subscribe(const std::string& key);
  /**
   * @brief unsubscribe will decrease the subscribtionCount of this key.
   * If the given key was subscribed multiple times it will continue to be subscribed (see
   * subscribe())
   */
  void unsubscribe(const std::string& key);

private:
  /**
   * @brief Used to retry subscribing unknown keys.
   */
  void resolveOutstandingSubscriptions();
  /// all transporters to notify when a new debugMap is available for transport.
  std::vector<std::shared_ptr<DebugTransportInterface>> transporter_;
  /// all debug sources to get updates from
  std::unordered_map<std::string, DebugSource> debugSources_;
  /// if a key could not be subscribed the debug module has to retry this again
  std::unordered_map<std::string, std::atomic<unsigned int>> outstandingSubscriptions_;

  /// used for thread synchronization
  std::thread transporterThread_;
  /// if the debug cycle was triggered from the outside
  bool trigger_ = false;
  /// if this thread should shutdown
  bool shutdownThread_ = false;
  /// for waking this thread up
  std::condition_variable transporterCondition_;
  /// the mutex used for reading/writing trigger_
  std::mutex transporterMutex_;

#ifdef ITTNOTIFY_FOUND
  /// the debug thread domain for vtune instrumentation
  __itt_domain* debugDomain_;
  /// the transporting debug task for vtune instrumentation
  __itt_string_handle* transportString_;
#endif
};
