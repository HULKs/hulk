/**
 * @file print.h
 * @brief This file provides some printing stuff
 * @author <a href="mailto:roboting@tuhh.de">RobotING@TUHH</a>
 */

#ifndef PRINT_H
#define PRINT_H

#include <cstdint>
#include <iostream>

#include "Modules/Log/Log.h"

/**
 * @namespace tuhhprint
 * @brief The tuhhprint namespace provides all the printing action functions.
 */
namespace tuhhprint{

  extern LogLevel minll;

  extern std::string preString[(int) LogLevel::LOG_LEVEL_MAX];

  extern void print(const std::string& message, const LogLevel& ll);

  extern void print(const std::string& message, const float& value, const LogLevel& ll);

  extern void print(const std::string& message, const std::string& value, const LogLevel &ll);

  extern void setLogLevel(LogLevel const& loglevel);

  extern LogLevel getLogLevel(const std::string& levelstr);

  typedef LogTemplate< M_TUHHSDK > Log;

}

using namespace tuhhprint;

#endif // PRINT_H
