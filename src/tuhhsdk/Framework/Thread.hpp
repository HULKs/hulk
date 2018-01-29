#pragma once

#include <memory>
#include <string>
#include <thread>

#include "Framework/Messaging.hpp"
#include "Modules/Log/Log.h"


class Configuration;
class Debug;
class RobotInterface;
class SharedObject;


struct ThreadData final
{
  /// the loglevel used for the thread
  LogLevel loglevel = LogLevel::ERROR;
  /// the list of senders
  std::vector<Sender*> senders;
  /// the list of receivers
  std::vector<Receiver*> receivers;
  /// the Debug instance
  Debug* debug = nullptr;
  /// the Configuration instance
  Configuration* configuration = nullptr;
  /// the RobotInterface instance
  RobotInterface* robotInterface = nullptr;
};


class ThreadBase
{
public:
  /**
   * @brief ThreadBase initializes members
   * @param tData the thread data for this thread
   */
  ThreadBase(ThreadData& tData);
  /**
   * @brief ~ThreadBase virtual destructor for polymorphism
   */
  virtual ~ThreadBase() = default;
  /**
   * @brief start creates the thread and runs it
   */
  void start();
  /**
   * @brief stop asks the thread to stop
   */
  void stop();
  /**
   * @brief join waits until the thread terminates
   */
  void join();
protected:
  /**
   * @brief init does some initialization inside the thread
   * @return true iff the initialization was successful
   */
  virtual bool init() = 0;
  /**
   * @brief cycle must be overridden to execute recurring tasks
   */
  virtual void cycle() = 0;
  /**
   * @brief triggerDebug triggers a debug transport cycle
   */
  void triggerDebug();
  /// reference to the thread data for this thread
  ThreadData& tData_;
private:
  /**
   * @brief main is the function that is executed inside the thread
   */
  void main();
  /// whether the thread should stop
  bool shouldStop_ = false;
  /// the actual thread handle
  std::thread thread_;
};

class ThreadFactoryBase
{
public:
  /**
   * @brief ThreadFactoryBase inserts the factory into the static list of factories
   */
  ThreadFactoryBase()
  {
    next = begin;
    begin = this;
  }
  /**
   * @brief ~ThreadFactoryBase virtual destructor for polymorphism
   */
  virtual ~ThreadFactoryBase() = default;
  /**
   * @brief use does nothing but is needed
   */
  void use()
  {
  }
  /**
   * @brief produce creates an instance of the thread class
   * @param data the thread data that belongs to this thread
   */
  virtual std::shared_ptr<ThreadBase> produce(ThreadData& data) const = 0;
  /**
   * @brief getName returns a human readable name of the thread type
   * @return a string containing the name of the thread type
   */
  virtual std::string getName() const = 0;
  /// next pointer in the factory linked list
  ThreadFactoryBase* next;
  /// the first element of the linked list
  static ThreadFactoryBase* begin;
};

template<typename T>
class ThreadFactory : public ThreadFactoryBase
{
public:
  /**
   * @brief produce creates an instance of the thread class
   * @param data the thread data that belongs to this thread
   */
  std::shared_ptr<ThreadBase> produce(ThreadData& data) const
  {
    return std::make_shared<T>(data);
  }
  /**
   * @brief getName returns a human readable name of the thread type
   * @return a string containing the name of the thread type
   */
  std::string getName() const
  {
    return T::getName();
  }
};

template<typename T>
class Thread : public ThreadBase
{
public:
  /**
   * @brief Thread calls use to instantiate things in the compiler
   * @param tData reference to the thread data for this thread
   */
  Thread(ThreadData& tData)
    : ThreadBase(tData)
  {
    factory_.use();
  }
private:
  /// the static factory
  static ThreadFactory<T> factory_;
};

template<typename T>
ThreadFactory<T> Thread<T>::factory_;
