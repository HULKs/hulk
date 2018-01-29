#include <string>

#include "Modules/Debug/Debug.h"

#include "Time.hpp"
#include "Chronometer.hpp"


Chronometer::Chronometer(Debug& debug, const std::string& key)
  : key_(key)
  , debug_(debug)
  , startTime_(getThreadTime())
{
}

Chronometer::~Chronometer()
{
  debug_.update(key_, static_cast<float>(getThreadTime() - startTime_) / 1000000000);
}
