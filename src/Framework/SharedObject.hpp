#pragma once

#include <memory>
#include <string>


class ThreadBase;
struct ThreadData;

/**
 * @class SharedObject is a wrapper for threads
 */
class SharedObject final
{
public:
  /**
   * @brief SharedObject constructs a thread with a given name
   * @param name the name of the thread
   * @param threadData a reference to the thread data for this shared object
   */
  SharedObject(const std::string& name, ThreadData& threadData);
  /**
   * @brief start starts the thread
   */
  void start();
  /**
   * @brief stop tells the thread to not enter its main loop again
   */
  void stop();
  /**
   * @brief join waits for the thread's termination
   */
  void join();

private:
  /// thread handle
  std::shared_ptr<ThreadBase> thread_;
  /// data used for the thread
  ThreadData& threadDatum_;
};
