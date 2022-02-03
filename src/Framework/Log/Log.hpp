#pragma once

#include "Hardware/Clock.hpp"
#include "Tools/Storage/UniValue/UniValue.h"
#include "Tools/Storage/UniValue/UniValue2JsonString.h"
#include <boost/algorithm/string.hpp>
#include <iostream>

enum ModuleCategory : uint8_t
{
  M_TUHHSDK,
  M_MOTION,
  M_VISION,
  M_BRAIN,
  M_MODULE_MAX = 4
};

enum class LogLevel
{
  VERBOSE,
  DEBUG,
  FANCY,
  INFO,
  WARNING,
  ERROR,
  LOG_LEVEL_MAX
};

template <ModuleCategory ID>
class Log
{
private:
  static const char* ModuleMap[M_MODULE_MAX];

  static LogLevel maxLogLevel_;
  LogLevel loglevel_;

  static std::string getFancy(std::string message)
  {
    srand(time(0));
    char pre[] = "123456";
    std::stringstream ss;
    ss.str(std::string());
    for (unsigned int i = 0; i < message.size(); i++)
    {
      ss << "\033[0;3" << pre[rand() % (sizeof(pre) - 1)] << "m" << message.at(i);
    }
    ss << "\033[0m ";
    return ss.str();
  }

public:
  Log(LogLevel loglevel)
    : loglevel_(loglevel)
  {
    if (loglevel_ < maxLogLevel_)
    {
      return;
    }

    std::cout << getPreString(loglevel);
  }

  ~Log()
  {
    if (loglevel_ < maxLogLevel_)
    {
      return;
    }
    std::cout << "\n";
  }

  static std::string getPreString(LogLevel loglevel)
  {
    std::string color;
    std::string level;
    const std::string module = ModuleMap[ID];

    switch (loglevel)
    {
      case LogLevel::VERBOSE:
        color = "\033[0;37m";
        level = "VERB";
        break;
      case LogLevel::DEBUG:
        color = "";
        level = "DEBUG";
        break;
      case LogLevel::FANCY:
        color = "\033[1;35m";
        level = "FANCY";
        return getFancy("[" + module + "_" + level + "]");
      case LogLevel::INFO:
        color = "";
        level = "INFO";
        break;
      case LogLevel::WARNING:
        color = "\033[0;33m";
        level = "WARN";
        break;
      case LogLevel::ERROR:
        color = "\033[0;31m";
        level = "ERROR";
      case LogLevel::LOG_LEVEL_MAX:
      default:
        break;
    }
    return color + "[" + module + "_" + level + "]\033[0m ";
  }

  Log& operator<<(Uni::Value& object)
  {
    if (loglevel_ >= maxLogLevel_)
    {
      std::cout << Uni::Converter::toJsonString(object);
    }

    return *this;
  }

  Log& operator<<(const std::string& text)
  {
    if (loglevel_ >= maxLogLevel_)
    {
      std::cout << text;
    }

    return *this;
  }

  Log& operator<<(const int& integer)
  {
    if (loglevel_ >= maxLogLevel_)
    {
      std::cout << integer;
    }

    return *this;
  }

  Log& operator<<(const unsigned int& unsignedInteger)
  {
    if (loglevel_ >= maxLogLevel_)
    {
      std::cout << unsignedInteger;
    }

    return *this;
  }

  Log& operator<<(const long& integer)
  {
    if (loglevel_ >= maxLogLevel_)
    {
      std::cout << integer;
    }

    return *this;
  }

  Log& operator<<(const unsigned long& unsignedInteger)
  {
    if (loglevel_ >= maxLogLevel_)
    {
      std::cout << unsignedInteger;
    }

    return *this;
  }

  Log& operator<<(const double& real)
  {
    if (loglevel_ >= maxLogLevel_)
    {
      std::cout << real;
    }

    return *this;
  }

  Log& operator<<(const Uni::To& value)
  {
    if (loglevel_ >= maxLogLevel_)
    {
      Uni::Value v;
      v << value;
      std::cout << Uni::Converter::toJsonString(v);
    }

    return *this;
  }

  static LogLevel getLogLevelFromLogLevel(int level)
  {
    if (level == 0)
    {
      return LogLevel::VERBOSE;
    }
    if (level == 1)
    {
      return LogLevel::DEBUG;
    }
    if (level == 2)
    {
      return LogLevel::FANCY;
    }
    if (level == 3)
    {
      return LogLevel::INFO;
    }
    if (level == 4)
    {
      return LogLevel::WARNING;
    }
    if (level == 5)
    {
      return LogLevel::ERROR;
    }
    return LogLevel::INFO;
  }

  static LogLevel getLogLevel(const std::string& levelstr)
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

  static void setLogLevel(int ll)
  {
    maxLogLevel_ = getLogLevelFromLogLevel(ll);
  }

  static void setLogLevel(LogLevel ll)
  {
    maxLogLevel_ = ll;
  }
};

template <ModuleCategory ID>
LogLevel Log<ID>::maxLogLevel_ = LogLevel::DEBUG;

template <ModuleCategory ID>
const char* Log<ID>::ModuleMap[M_MODULE_MAX] = {"TUHH", "MOTION", "VISION", "BRAIN"};
