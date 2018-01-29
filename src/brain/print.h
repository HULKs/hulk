#ifndef PRINT_H
#define PRINT_H

#include "Modules/Log/Log.h"

namespace brainprint{
  extern LogLevel minll;

  extern std::string preString[(int) LogLevel::LOG_LEVEL_MAX];

  void print(const std::string& message, const LogLevel& ll);

  void print(const std::string& message, const float& value, const LogLevel& ll);

  void setLogLevel(LogLevel const& loglevel);

  typedef LogTemplate< M_BRAIN > Log;
}
using namespace brainprint;

#endif //PRINT_H
