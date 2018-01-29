#include <ctime>
#include <iostream>
#include <sstream>

#include "print.h"


LogLevel brainprint::minll = LogLevel::DEBUG;

static std::string getFancy()
{
  srand(time(0));
  char msg[] = "[BRAIN_FANCY\t]";
  char pre[] = "123456";
  std::stringstream ss;
  ss.str(std::string());
  for(unsigned int i = 0; i < sizeof(msg); i++)
  {
    ss << "\033[0;3" << pre[rand()%(sizeof(pre)-1)] << "m" << msg[i];
  }
  ss << "\033[0m ";
  return ss.str();
}

std::string brainprint::preString[(int)LogLevel::LOG_LEVEL_MAX] =
{
  "[BRAIN_VERBOSE\t] ",
  "[BRAIN_DEBUG\t] ",
  getFancy(),
  "[BRAIN_INFO\t] ",
  "\033[0;33m[BRAIN_WARN\t]\033[0m ",
  "\033[0;31m[BRAIN_ERROR\t]\033[0m "
};

void brainprint::print(const std::string& message, const LogLevel &ll)
{
  if (minll <= ll)
    std::cout << preString[(int)ll] << message << "\n";
}

void brainprint::print(const std::string& message, const float& value, const LogLevel& ll)
{
  if (minll <= ll)
    std::cout << preString[(int)ll] << message << ' ' << value << "\n";
}

void brainprint::setLogLevel(const LogLevel &loglevel)
{
  minll = loglevel;
  Log::setLogLevel(loglevel);
}
