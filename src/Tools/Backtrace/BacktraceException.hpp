/* Copyright (c) 2009, Fredrik Orderud
   License: BSD licence (http://www.opensource.org/licenses/bsd-license.php)

   From: https://sourceforge.net/p/stacktrace/code/HEAD/tree/stacktrace/stack_exception.hpp
*/

#pragma once

#include <stdexcept>
#include <string>

#include "Tools/Backtrace/Backtrace.hpp"

/** Abstract base-class for all stack-augmented exception classes.
 *  Enables catching of all stack-augmented exception classes. */
class stack_exception_base
{
public:
  stack_exception_base() = default;
  virtual ~stack_exception_base() noexcept = default;

  virtual const char* what() const noexcept = 0;
};

/** Template for stack-augmented exception classes. */
template <class T>
class stack_exception : public T, public stack_exception_base
{
public:
  explicit stack_exception(const std::string& msg)
    : T(msg)
    , stack_exception_base()
    , trace_(backtrace())
  {
  }
  ~stack_exception() noexcept override = default;

  const char* what() const noexcept override
  {
    // concatenate message with stack trace
    buffer_ = "[" + std::string(T::what()) + "]\n" + trace_;
    return buffer_.c_str();
  }

private:
  mutable std::string buffer_;
  std::string trace_;
};

/** Stack-augmented exception classes for all std::exception classes. */
typedef stack_exception<std::runtime_error> stack_runtime_error;
typedef stack_exception<std::range_error> stack_range_error;
typedef stack_exception<std::overflow_error> stack_overflow_error;
typedef stack_exception<std::underflow_error> stack_underflow_error;
typedef stack_exception<std::logic_error> stack_logic_error;
typedef stack_exception<std::domain_error> stack_domain_error;
typedef stack_exception<std::invalid_argument> stack_invalid_argument;
typedef stack_exception<std::length_error> stack_length_error;
typedef stack_exception<std::out_of_range> stack_out_of_range;
