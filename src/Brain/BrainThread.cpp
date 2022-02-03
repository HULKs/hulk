#include "Framework/Configuration/Configuration.h"

#include "Brain/Brain.hpp"
#include "Framework/Log/Log.hpp"

#include "Brain/BrainThread.hpp"


BrainThread::BrainThread(ThreadData& data)
  : Thread(data)
{
  Log<M_BRAIN>(LogLevel::INFO) << "module_init()";
  Log<M_BRAIN>(LogLevel::INFO) << "LogLevel is set to "
                               << Log<M_BRAIN>::getPreString(Log<M_BRAIN>::getLogLevelFromLogLevel(
                                      static_cast<int>(threadData_.loglevel)));
  /// init variables
  Log<M_BRAIN>::setLogLevel(threadData_.loglevel);
  Log<M_VISION>::setLogLevel(threadData_.loglevel);
  try
  {
    brain_ = std::make_shared<Brain>(threadData_.senders, threadData_.receivers, *threadData_.debug,
                                     *threadData_.configuration, *threadData_.robotInterface);
  }
  catch (const std::exception& e)
  {
    Log<M_BRAIN>(LogLevel::ERROR) << e.what();
    throw std::runtime_error("Brain could not be initialized");
  }
  catch (...)
  {
    Log<M_BRAIN>(LogLevel::ERROR) << "Exception in Brain constructor";
    throw;
  }

  Log<M_BRAIN>(LogLevel::INFO) << "module_init() ... done";
}

bool BrainThread::init()
{
  if (!brain_)
  {
    Log<M_BRAIN>(LogLevel::ERROR) << "brain is NULL and cannot run.";
    return false;
  }
#ifdef ITTNOTIFY_FOUND
  __itt_thread_set_name("Brain");
#endif
  return true;
}

void BrainThread::cycle()
{
  try
  {
    brain_->runCycle();
  }
  catch (const std::exception& e)
  {
    Log<M_BRAIN>(LogLevel::ERROR) << "Brain, module_main";
    Log<M_BRAIN>(LogLevel::ERROR) << e.what();
    throw;
  }
  catch (...)
  {
    Log<M_BRAIN>(LogLevel::ERROR) << "Unknown exception in BrainModule module_main()";
    throw;
  }
}
