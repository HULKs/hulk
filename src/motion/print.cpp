#include <ctime>
#include <iostream>
#include <sstream>

#include "print.hpp"

LogLevel motionprint::minll = LogLevel::DEBUG;

std::string getFancy()
{
  srand(time(0));
  char msg[] = "[MOTION_FANCY\t]";
  char pre[] = "123456";
  std::stringstream ss;
  ss.str(std::string());
  for(unsigned int i = 0; i < sizeof(msg); i++)
  {
    ss << "\033[0;3" << pre[rand()%(sizeof(pre)-1)] << "m" << msg[i];
  }
  ss << "\033[0;29m ";
  return ss.str();
}

std::string motionprint::preString[(int)LogLevel::LOG_LEVEL_MAX] =
{
  "[MOTION_VERBOSE\t] ",
  "[MOTION_DEBUG\t] ",
  getFancy(),
  "[MOTION_INFO\t] ",
  "\033[0;33m[MOTION_WARN\t]\033[0m ",
  "\033[0;31m[MOTION_ERROR\t]\033[0m "
};

void motionprint::print(const std::string& message, const LogLevel& ll)
{
  if (minll <= ll)
    std::cout << preString[(int)ll] << message << "\n";
}

void motionprint::print(const std::string& message, const float& value, const LogLevel& ll)
{
  if (minll <= ll)
    std::cout << preString[(int)ll] << message << ' ' << value << "\n";
}

void motionprint::setLogLevel(LogLevel const& loglevel)
{
  minll = loglevel;
  Log::setLogLevel(loglevel);
}
