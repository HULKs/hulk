#pragma once

#include <algorithm>
#include <chrono>
#include <ctime>
#include <execution>
#include <iomanip>
#include <iostream>
#include <map>
#include <mutex>
#include <sstream>
#include <thread>
#include <vector>

namespace Hulks::Runner
{

  template <typename Processor>
  class Runner
  {
  public:
    template <typename UserData>
    void runUntilComplete(const UserData& userData)
    {
      using namespace std::chrono_literals;

      const auto beginOfRun = std::chrono::system_clock::now();

      std::vector<typename Processor::ItemType> items{Processor::prologue(*this, userData)};

      done_ = 0;
      total_ = items.size();
      beginInitialized_ = false;
      beginOfProcessing_ = std::chrono::system_clock::now();

      std::for_each(std::execution::par_unseq, items.begin(), items.end(),
                    [this, &userData](const typename Processor::ItemType& item) {
                      {
                        std::lock_guard lock{beginMutex_};
                        if (!beginInitialized_)
                        {
                          beginInitialized_ = true;
                          beginOfProcessing_ = std::chrono::system_clock::now();
                        }
                      }

                      getProcessor(userData).process(item);

                      {
                        std::lock_guard lock{outputMutex_};
                        ++done_;
                        updateProgressLine();
                      }
                    });

      const auto endOfProcessing = std::chrono::system_clock::now();

      std::lock_guard lock{processorsMutex_};
      processors_.clear();

      Processor::epilogue(*this, userData);

      const auto endOfRun = std::chrono::system_clock::now();

      // output final progress line with a summary
      std::cout << '\r';
      for (auto i = 0; i < currentProgressLineLength_; ++i)
      {
        std::cout << " ";
      }
      std::cout << std::flush;
      std::cout << "\r\n";

      std::cout << "Processed " << total_ << " items, "
                << formatDuration(total_ == 0 ? 0s
                                              : (endOfProcessing - beginOfProcessing_) / total_)
                << " each\n\n";
      std::cout << "+----------- run -- " << formatTimePoint(beginOfRun) << '\n';
      std::cout << "|                    " << formatDuration(beginOfProcessing_ - beginOfRun)
                << "  prologue\n";
      std::cout << "|  +- processing -- " << formatTimePoint(beginOfProcessing_) << '\n';
      std::cout << "|  |\n";
      std::cout << "|  |                 " << formatDuration(endOfProcessing - beginOfProcessing_)
                << "  processing\n";
      std::cout << "|  |\n";
      std::cout << "|  +--------------- " << formatTimePoint(endOfProcessing) << '\n';
      std::cout << "|                    " << formatDuration(endOfRun - endOfProcessing)
                << "  epilogue\n";
      std::cout << "+------------------ " << formatTimePoint(endOfRun) << "\n\n" << std::flush;
    }

    void writeLog(const std::string& logLine)
    {
      std::lock_guard lock{outputMutex_};

      // rewind progress line
      std::cout << '\r';

      // output log line with timestamp
      const auto dateTime = formatTimePoint(std::chrono::system_clock::now());
      std::cout << dateTime << ": " << logLine;

      const auto logLineLength = dateTime.size() + 2 + logLine.size();
      for (auto i = logLineLength; i < currentProgressLineLength_; ++i)
      {
        std::cout << " ";
      }
      std::cout << '\n';

      // write current progress line
      currentProgressLineLength_ = outputProgressLine();
      std::cout << std::flush;
    }

  private:
    std::string formatDuration(const std::chrono::system_clock::duration& duration)
    {
      // TODO: revert manual definitions of days, months, years when switching to C++20
      using years = std::chrono::duration<int64_t, std::ratio<31556952>>;
      using months = std::chrono::duration<int64_t, std::ratio<2629746>>;
      using days = std::chrono::duration<int64_t, std::ratio<86400>>;
      using hours = std::chrono::hours;
      using minutes = std::chrono::minutes;
      using seconds = std::chrono::seconds;
      using milliseconds = std::chrono::milliseconds;
      using microseconds = std::chrono::microseconds;

      if (duration >= years{1})
      {
        const auto durationYears = std::chrono::duration_cast<years>(duration);
        return std::to_string(durationYears.count()) + "y " +
               std::to_string(
                   std::chrono::duration_cast<months>(duration - durationYears).count()) +
               "m";
      }

      if (duration >= months{1})
      {
        const auto durationMonths = std::chrono::duration_cast<months>(duration);
        return std::to_string(durationMonths.count()) + "m " +
               std::to_string(std::chrono::duration_cast<days>(duration - durationMonths).count()) +
               "d";
      }

      if (duration >= days{1})
      {
        const auto durationDays = std::chrono::duration_cast<days>(duration);
        return std::to_string(durationDays.count()) + "d " +
               std::to_string(std::chrono::duration_cast<hours>(duration - durationDays).count()) +
               "h";
      }

      if (duration >= hours{1})
      {
        const auto durationHours = std::chrono::duration_cast<hours>(duration);
        return std::to_string(durationHours.count()) + "h " +
               std::to_string(
                   std::chrono::duration_cast<minutes>(duration - durationHours).count()) +
               "min";
      }

      if (duration >= minutes{1})
      {
        const auto durationMinutes = std::chrono::duration_cast<minutes>(duration);
        return std::to_string(durationMinutes.count()) + "min " +
               std::to_string(
                   std::chrono::duration_cast<seconds>(duration - durationMinutes).count()) +
               "s";
      }

      if (duration >= seconds{1})
      {
        const auto durationSeconds = std::chrono::duration_cast<seconds>(duration);
        return std::to_string(durationSeconds.count()) + "s " +
               std::to_string(
                   std::chrono::duration_cast<milliseconds>(duration - durationSeconds).count()) +
               "ms";
      }

      if (duration >= milliseconds{1})
      {
        const auto durationMilliseconds = std::chrono::duration_cast<milliseconds>(duration);
        return std::to_string(durationMilliseconds.count()) + "ms " +
               std::to_string(
                   std::chrono::duration_cast<microseconds>(duration - durationMilliseconds)
                       .count()) +
               "us";
      }

      return std::to_string(std::chrono::duration_cast<microseconds>(duration).count()) + "us";
    }

    std::string formatTimePoint(const std::chrono::system_clock::time_point& timePoint)
    {
      const auto nowTimeT = std::chrono::system_clock::to_time_t(timePoint);
      const auto* nowTm = std::localtime(&nowTimeT);
      std::stringstream dateTimeStream;
      dateTimeStream << std::put_time(nowTm, "%F %T");
      return dateTimeStream.str();
    }

    void updateProgressLine()
    {
      // rewind progress line
      std::cout << '\r';

      // write current progress line
      const auto progressLineLength = outputProgressLine();
      for (auto i = progressLineLength; i < currentProgressLineLength_; ++i)
      {
        std::cout << " ";
      }
      std::cout << std::flush;
      currentProgressLineLength_ = progressLineLength;
    }

    std::size_t outputProgressLine()
    {
      using namespace std::chrono_literals;

      const auto now = std::chrono::system_clock::now();
      const auto totalDuration = now - beginOfProcessing_;
      const auto averageDuration = done_ == 0 ? 0s : totalDuration / done_;
      const auto remainingEstimate = averageDuration * (total_ - done_);
      const auto estimatedEnd = now + remainingEstimate;

      std::stringstream progressLine;
      progressLine << done_ << "/" << total_ << ", " << formatDuration(averageDuration)
                   << " each, ca. " << formatDuration(remainingEstimate) << " remaining, ca. "
                   << formatTimePoint(estimatedEnd) << " finished";

      std::cout << progressLine.str();

      return progressLine.str().size();
    }

    template <typename UserData>
    Processor& getProcessor(const UserData& userData)
    {
      std::lock_guard lock{processorsMutex_};
      return processors_.try_emplace(std::this_thread::get_id(), *this, userData).first->second;
    }

    std::size_t currentProgressLineLength_{0};
    std::mutex processorsMutex_;
    std::map<std::thread::id, Processor> processors_;

    std::mutex outputMutex_;
    std::size_t done_{0};
    std::size_t total_{0};

    std::mutex beginMutex_;
    bool beginInitialized_{false};
    std::chrono::time_point<std::chrono::system_clock> beginOfProcessing_;
  };

} // namespace Hulks::Runner
