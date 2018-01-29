#include "WhistleDetection.hpp"
#include "Tools/Chronometer.hpp"
#include "print.h"

WhistleDetection::WhistleDetection(const ModuleManagerInterface& manager)
  : Module(manager, "WhistleDetection")
  , recordData_(*this)
  , rawGameControllerState_(*this)
  , cycleInfo_(*this)
  , whistleData_(*this)
  , minFrequency_(*this, "minFrequency", [] {})
  , maxFrequency_(*this, "maxFrequency", [] {})
  , threshold_(*this, "threshold", [] {})
  , fft_(BUFFER_SIZE)
  , lastTimeWhistleHeard_()
{
}

void WhistleDetection::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  if (rawGameControllerState_->state != GameState::SET)
  {
    return;
  }
  if (recordData_->samples.empty())
  {
    return;
  }

  for (auto& sample : recordData_->samples)
  {
    buffer_.push_back(sample);

    if (buffer_.size() == BUFFER_SIZE)
    {
      if (bufferContainsWhistle())
      {
        lastTimeWhistleHeard_ = cycleInfo_->startTime;
      }
      buffer_.clear();
      break;
    }
  }

  whistleData_->lastTimeWhistleHeard = lastTimeWhistleHeard_;
}

bool WhistleDetection::bufferContainsWhistle()
{
  auto freqData = fft_.fft(buffer_);

  double freqResolution = samplingRate / BUFFER_SIZE;

  unsigned int minFreqIndex = ceil(minFrequency_() / freqResolution);
  unsigned int maxFreqIndex = ceil(maxFrequency_() / freqResolution);

  if (maxFreqIndex > BUFFER_SIZE)
  {
    throw std::runtime_error("WhistleDetection: maxFrequency can not be higher than nyquist frequency.");
  }

  double power = 0;
  double stopBandPower = 0;
  for (unsigned int i = minFreqIndex; i < freqData.size(); ++i)
  {
    if (i < maxFreqIndex)
    {
      power += std::abs(freqData[i]) * std::abs(freqData[i]) * freqResolution;
    }
    else
    {
      stopBandPower += std::abs(freqData[i]) * std::abs(freqData[i]) * freqResolution;
    }
  }

  Log(LogLevel::DEBUG) << "Flötenpower: " << power;
  Log(LogLevel::DEBUG) << "Non-Flötenpower: " << stopBandPower;
  Log(LogLevel::DEBUG) << "Difference" << power - stopBandPower;
  Log(LogLevel::DEBUG) << "Quotient" << power / stopBandPower;
  Log(LogLevel::DEBUG) << "-------------------";

  return power / stopBandPower > threshold_();
}
