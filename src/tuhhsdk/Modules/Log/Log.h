#ifndef LOG_H
#define LOG_H

#include <Tools/Storage/UniValue/UniValue.h>
#include <Tools/Storage/UniValue/UniValue2JsonString.h>

#include <iostream>

#include "Definitions/windows_definition_fix.hpp"

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
class LogTemplate
{
private:
  static const char* ModuleMap[M_MODULE_MAX];

  static LogLevel maxLogLevel_;
  LogLevel loglevel_;

public:
  LogTemplate(LogLevel loglevel)
    : loglevel_(loglevel)
  {
    if (loglevel_ < maxLogLevel_)
      return;

    std::string color;
    std::string level;

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
        break;
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

    std::cout << color << "[" << ModuleMap[ID] << "_" << level << "\t]\033[0m ";
  }

  ~LogTemplate()
  {
    if (loglevel_ < maxLogLevel_)
      return;
    std::cout << "\n";
  }

  LogTemplate& operator<<(Uni::Value& object)
  {
    if (loglevel_ >= maxLogLevel_)
    {
      std::cout << Uni::Converter::toJsonString(object);
    }

    return *this;
  }

  LogTemplate& operator<<(const std::string& text)
  {
    if (loglevel_ >= maxLogLevel_)
    {
      std::cout << text;
    }

    return *this;
  }

  LogTemplate& operator<<(const int& integer)
  {
    if (loglevel_ >= maxLogLevel_)
    {
      std::cout << integer;
    }

    return *this;
  }

  LogTemplate& operator<<(const double& real)
  {
    if (loglevel_ >= maxLogLevel_)
    {
      std::cout << real;
    }

    return *this;
  }

  LogTemplate& operator<<(const Uni::To& value)
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
LogLevel LogTemplate<ID>::maxLogLevel_ = LogLevel::DEBUG;

template <ModuleCategory ID>
const char* LogTemplate<ID>::ModuleMap[M_MODULE_MAX] = {"TUHH", "MOTION", "VISION", "BRAIN"};

#endif // LOG_H
