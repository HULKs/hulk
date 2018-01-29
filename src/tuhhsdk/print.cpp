#include <sstream>
#include <random>

#include <boost/algorithm/string.hpp>

#include "print.h"

LogLevel tuhhprint::minll = LogLevel::DEBUG;

static std::string getFancy()
{
  std::random_device rnd;
  std::mt19937 rng(rnd());
  char msg[] = "[TUHH_FANCY]";
  std::uniform_int_distribution<int> dis(1,6);
  std::stringstream ss;
  ss.str(std::string());
  for(unsigned int i = 0; i < sizeof(msg)-2; i++)
  {
    ss << "\033[0;3" << dis(rng) << "m" << msg[i];
  }
  ss << "\t" << "\033[0;3" << dis(rng) << "m" << msg[sizeof(msg)-2] << "\033[0;29m ";
  return ss.str();
}

std::string tuhhprint::preString[((int) LogLevel::LOG_LEVEL_MAX)] =
{
  "[TUHH_VERBOSE\t] ",
  "[TUHH_DEBUG\t] ",
  getFancy(),
  "[TUHH_INFO\t] ",
  "\033[0;33m[TUHH_WARN\t]\033[0m ",
  "\033[0;31m[TUHH_ERROR\t]\033[0m "
};

void tuhhprint::print(const std::string& message, const LogLevel& ll)
{
  if (minll <= ll)
    std::cout << tuhhprint::preString[(int) ll] << message << "\n";
}

void tuhhprint::print(const std::string& message, const float& value, const LogLevel& ll)
{
  std::cout.precision(6);
  if (minll <= ll)
    std::cout << preString[(int) ll] << message << ' ' << value << "\n";
}

void tuhhprint::print(const std::string& message, const std::string& value, const LogLevel& ll)
{
  if (minll <= ll)
    std::cout << preString[(int) ll] << message << ' ' << value << "\n";
}

void tuhhprint::setLogLevel(const LogLevel &loglevel)
{
  minll = loglevel;
  Log::setLogLevel(loglevel);
}

LogLevel tuhhprint::getLogLevel(const std::string& levelstr)
{
  if (boost::iequals(levelstr, "debug"))
  {
    return LogLevel::DEBUG;
  }
  else if (boost::iequals(levelstr, "fancy"))
  {
    return LogLevel::FANCY; // I'm so fancy.
  }
  else if (boost::iequals(levelstr, "info"))
  {
    return LogLevel::INFO;
  }
  else if (boost::iequals(levelstr, "warning"))
  {
    return LogLevel::WARNING;
  }
  else if (boost::iequals(levelstr, "error"))
  {
    return LogLevel::ERROR;
  }
  else
  {
    return LogLevel::INFO;
  }
}
