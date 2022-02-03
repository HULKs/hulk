#include <cassert>
#include <mutex>

#include "Framework/Thread.hpp"

#include "Framework/Log/Log.hpp"

#include "Framework/SharedObject.hpp"


SharedObject::SharedObject(const std::string& name, ThreadData& threadData)
  : thread_()
  , threadDatum_(threadData)
{
  ThreadFactoryBase* factory;
  for (factory = ThreadFactoryBase::begin; factory != nullptr; factory = factory->next)
  {
    Log<M_TUHHSDK>(LogLevel::DEBUG) << factory->getName();
    if (factory->getName() == name)
    {
      thread_ = factory->produce(threadDatum_);
      break;
    }
  }
  assert(factory != nullptr);
}

void SharedObject::start()
{
  thread_->start();
}

void SharedObject::stop()
{
  thread_->stop();
}

void SharedObject::join()
{
  thread_->join();
}
