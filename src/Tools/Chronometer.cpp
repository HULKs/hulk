#include "Tools/Chronometer.hpp"
#include <string>

Chronometer::Chronometer(DebugDatabase::DebugMap& debug, std::string key)
  : key_(std::move(key))
  , debug_(debug)
  , startTime_(getThreadTime())
{
}

Chronometer::~Chronometer()
{
  stop();
}

void Chronometer::stop()
{
  bool expected = false;
  if (isStopped_.compare_exchange_weak(expected, true))
  {
    debug_.update(key_, static_cast<float>(getThreadTime() - startTime_) / 1'000'000'000.f);
    return;
  }
}
