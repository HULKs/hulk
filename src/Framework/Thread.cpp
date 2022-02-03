#include <exception>

#include "Framework/Debug/Debug.h"
#include "Framework/Log/Log.hpp"

#include "Framework/Thread.hpp"


ThreadFactoryBase* ThreadFactoryBase::begin = nullptr;

ThreadBase::ThreadBase(ThreadData& threadData)
  : threadData_(threadData)
{
}

void ThreadBase::start()
{
  shouldStop_ = false;
  thread_ = std::thread([this] { main(); });
}

void ThreadBase::stop()
{
  shouldStop_ = true;
}

void ThreadBase::join()
{
  if (thread_.joinable())
  {
    thread_.join();
  }
}

void ThreadBase::main()
{
  Log<M_TUHHSDK>(LogLevel::INFO) << "Starting main thread";
  try
  {
    if (!init())
    {
      return;
    }
    while (!shouldStop_)
    {
      cycle();
    }
    Log<M_TUHHSDK>(LogLevel::INFO) << "Shutting down thread";
  }
  catch (const std::exception& e)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "Uncaught exception in a thread: " << e.what();
    abort();
  }
  catch (...)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "Uncaught exception in a thread";
    abort();
  }
}
