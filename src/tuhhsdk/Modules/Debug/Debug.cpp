#include <functional>

#include "Debug.h"
#include "Tools/Storage/Image.hpp"


void Debug::updateHelper(const std::string& key, const Uni::Value& value)
{
  std::lock_guard<std::mutex> l(debugMutex_);
  DebugData dat(key, value);

  if (transporter_.size())
  {
    for (auto it = transporter_.begin(); it != transporter_.end(); it++)
    {
      (*it)->update(dat);
    }
  }
}

void Debug::subscribe(const std::string& key)
{
  std::lock_guard<std::mutex> l(keysMutex_);
  auto iter = keys_.find(key);
  if (iter != keys_.end())
  {
    (iter->second)++;
  }
  else
  {
    keys_[key] = 1;
  }
}

void Debug::unsubscribe(const std::string& key)
{
  std::lock_guard<std::mutex> l(keysMutex_);
  auto iter = keys_.find(key);
  assert(iter != keys_.end());
  assert(iter->second > 0);
  --(iter->second);
}

void Debug::pushQueue(const std::string& key, const std::string& message)
{
  if (transporter_.size())
  {
    for (auto it = transporter_.begin(); it != transporter_.end(); it++)
    {
      (*it)->pushQueue(key, message);
    }
  }
}

void Debug::sendImage(const std::string& key, const Image& img)
{
  {
    std::lock_guard<std::mutex> l(keysMutex_);
    auto it = keys_.find(key);
    if (it == keys_.end())
    {
      keys_[key] = 0;
    }
    else if (it->second == 0)
    {
      return;
    }
  }
  for (auto it = transporter_.begin(); it != transporter_.end(); it++)
  {
    (*it)->sendImage(key, img);
  }
}

void Debug::addTransport(const std::shared_ptr<DebugTransportInterface>& transport)
{
  transporter_.push_back(transport);
}

void Debug::removeAllTransports()
{
  transporter_.clear();
}

void Debug::start()
{
  if (transporter_.empty())
  {
    return;
  }
  shutdownThread_ = false;
  transporterThread_ = std::thread(std::bind(&Debug::run, this));
}

void Debug::stop()
{
  if (transporterThread_.joinable())
  {
    shutdownThread_ = true;
    trigger();
    transporterThread_.join();
  }
}

void Debug::run()
{
  while (true)
  {
    {
      std::unique_lock<std::mutex> l2(transporterMutex_);
      transporterCondition_.wait(l2, [this] { return trigger_; });
      if (shutdownThread_)
      {
        break;
      }
      trigger_ = false;
    }
    std::lock_guard<std::mutex> l(debugMutex_);
    for (auto transporter : transporter_)
    {
      transporter->transport();
    }
  }
}

void Debug::trigger()
{
  {
    std::lock_guard<std::mutex> lg(transporterMutex_);
    trigger_ = true;
  }
  transporterCondition_.notify_one();
}
