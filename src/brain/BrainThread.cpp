#include "Modules/Configuration/Configuration.h"

#include "Brain.hpp"
#include "print.h"

#include "BrainThread.hpp"


BrainThread::BrainThread(ThreadData& data) :
  Thread(data)
{
  print("module_init()", LogLevel::INFO);
  print("loglevel is set to: " + preString[(int)tData_.loglevel], LogLevel::INFO);
  /// init variables
  setLogLevel(tData_.loglevel);
  LogTemplate<M_VISION>::setLogLevel(tData_.loglevel);
  try
  {
    brain_ = std::make_shared<Brain>(tData_.senders, tData_.receivers, *tData_.debug, *tData_.configuration, *tData_.robotInterface);
  }
  catch (const std::exception& e)
  {
    print(e.what(), LogLevel::ERROR);
    return;
  }
  catch (...)
  {
    print("Exception in Brain constructor!", LogLevel::ERROR);
    return;
  }

  print("module_init() ... done", LogLevel::INFO);
}

bool BrainThread::init()
{
  if (!brain_)
  {
    print("brain is NULL and cannot run.", LogLevel::ERROR);
    return false;
  }
  return true;
}

void BrainThread::cycle()
{
  try
  {
    brain_->cycle();
  }
  catch (const std::exception& e)
  {
    print("Brain, module_main", LogLevel::ERROR);
    print(e.what(), LogLevel::ERROR);
  }
  catch (...)
  {
    print("Unknown exception in BrainModule module_main()", LogLevel::ERROR);
  }
}
