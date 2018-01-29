#include <exception>

#include "Modules/Debug/Debug.h"
#include "print.h"

#include "Thread.hpp"


ThreadFactoryBase* ThreadFactoryBase::begin = nullptr;

ThreadBase::ThreadBase(ThreadData& tData)
  : tData_(tData)
{
}

void ThreadBase::start()
{
  shouldStop_ = false;
  thread_ = std::thread([this]{main();});
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

void ThreadBase::triggerDebug()
{
  tData_.debug->trigger();
}

void ThreadBase::main()
{
  Log(LogLevel::INFO) << "Starting main thread!";
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
    Log(LogLevel::INFO) << "Shutting down thread!";
  }
  catch (const std::exception& e)
  {
    Log(LogLevel::ERROR) << "Uncaught exception in a thread: " << e.what();
  }
  catch (...)
  {
    Log(LogLevel::ERROR) << "Uncaught exception in a thread!";
  }
}
