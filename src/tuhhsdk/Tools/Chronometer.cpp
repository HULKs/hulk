#include <string>

#include "Time.hpp"
#include "Chronometer.hpp"


Chronometer::Chronometer(DebugDatabase::DebugMap& debug, const std::string& key)
  : key_(key)
  , debug_(debug)
  , startTime_(getThreadTime())
{
}

Chronometer::~Chronometer()
{
  debug_.update(key_, static_cast<float>(getThreadTime() - startTime_) / 1000000000);
}
