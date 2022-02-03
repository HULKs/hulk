#include <fenv.h>

#include "Framework/Log/Log.hpp"
#include "Motion/Motion.hpp"

#include "Motion/MotionThread.hpp"

MotionThread::MotionThread(ThreadData& data)
  : Thread(data)
{
  Log<M_MOTION>(LogLevel::INFO) << "module_init()";
  Log<M_MOTION>(LogLevel::INFO) << "LogLevel is set to "
                                << Log<M_MOTION>::getPreString(
                                       Log<M_MOTION>::getLogLevelFromLogLevel(
                                           static_cast<int>(threadData_.loglevel)));
  Log<M_MOTION>::setLogLevel(threadData_.loglevel);

  /// initialize motion
  try
  {
    motion_ =
        std::make_shared<Motion>(threadData_.senders, threadData_.receivers, *threadData_.debug,
                                 *threadData_.configuration, *threadData_.robotInterface);
  }
  catch (const std::exception& e)
  {
    Log<M_MOTION>(LogLevel::ERROR) << e.what();
    throw std::runtime_error("Motion could not be initialized");
  }
  catch (...)
  {
    Log<M_MOTION>(LogLevel::ERROR) << "Exception in Motion constructor";
    throw;
  }
}

bool MotionThread::init()
{
  // feenableexcept(FE_DIVBYZERO | FE_INVALID | FE_OVERFLOW);

  if (!motion_)
  {
    Log<M_MOTION>(LogLevel::ERROR) << "motion is NULL and cannot run";
    return false;
  }

#ifdef ITTNOTIFY_FOUND
  __itt_thread_set_name("Motion");
#endif

  // Set a real time priority for motion. 30 is still below the priority of the DCM and HAL threads
  // from naoqi.
  sched_param param;
  param.sched_priority = 30;
  pthread_setschedparam(pthread_self(), SCHED_FIFO, &param);
  return true;
}

void MotionThread::cycle()
{
  try
  {
    motion_->runCycle();
  }
  catch (const std::exception& e)
  {
    Log<M_MOTION>(LogLevel::ERROR) << e.what();
    throw;
  }
  catch (...)
  {
    Log<M_MOTION>(LogLevel::ERROR) << "Unknown exception in module_main()";
    throw;
  }
}
