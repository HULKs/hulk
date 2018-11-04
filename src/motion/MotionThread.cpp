#include <fenv.h>

#include "Motion.hpp"
#include "print.hpp"

#include "MotionThread.hpp"

MotionThread::MotionThread(ThreadData& data)
  : Thread(data)
{
  print("module_init()", LogLevel::INFO);
  print("LogLevel is set to " + preString[(int)tData_.loglevel], LogLevel::INFO);
  setLogLevel(tData_.loglevel);

  /// initialize motion
  try
  {
    motion_ = std::make_shared<Motion>(tData_.senders, tData_.receivers, *tData_.debug, *tData_.configuration, *tData_.robotInterface);
  }
  catch (const std::exception& e)
  {
    print(e.what(), LogLevel::ERROR);
    throw std::runtime_error("Motion could not be initialized");
  }
  catch (...)
  {
    print("Exception in Motion constructor!", LogLevel::ERROR);
    throw;
  }
}

bool MotionThread::init()
{
  // feenableexcept(FE_DIVBYZERO | FE_INVALID | FE_OVERFLOW);

  if (!motion_)
  {
    print("motion is NULL and cannot run!", LogLevel::ERROR);
    return false;
  }

#ifdef ITTNOTIFY_FOUND
  __itt_thread_set_name("Motion");
#endif

#ifndef WIN32
  // Set a real time priority for motion. 30 is still below the priority of the DCM and HAL threads from naoqi.
  sched_param param;
  param.sched_priority = 30;
  pthread_setschedparam(pthread_self(), SCHED_FIFO, &param);
#endif // !WIN32
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
    print(e.what(), LogLevel::ERROR);
  }
  catch (...)
  {
    print("Unknown exception in module_main()!", LogLevel::ERROR);
  }
}
