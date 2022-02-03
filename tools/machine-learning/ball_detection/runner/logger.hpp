#pragma once

#include "runner.hpp"
#include <sstream>
#include <utility>

namespace Hulks::Runner
{

  template <typename Processor>
  class Logger
  {
  public:
    explicit Logger(Runner<Processor>& runner)
      : runner_{runner}
    {
    }

    Logger(const Logger&) = delete;
    Logger& operator=(const Logger&) = delete;
    Logger(Logger&&) = delete;
    Logger& operator=(Logger&&) = delete;

    ~Logger()
    {
      runner_.writeLog(stream_.str());
    }

    template <typename ValueType>
    Logger<Processor>& operator<<(ValueType&& value)
    {
      stream_ << std::forward<ValueType>(value);
      return *this;
    }

  private:
    Runner<Processor>& runner_;
    std::stringstream stream_;
  };

} // namespace Hulks::Runner
