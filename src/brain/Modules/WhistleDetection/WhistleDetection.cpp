#include <numeric>

#include "Tools/Chronometer.hpp"
#include "Tools/Math/Statistics.hpp"
#include "print.h"

#include "WhistleDetection.hpp"

WhistleDetection::WhistleDetection(const ModuleManagerInterface& manager)
  : Module(manager)
  , recordData_(*this)
  , rawGameControllerState_(*this)
  , cycleInfo_(*this)
  , whistleData_(*this)
  , minFrequency_(*this, "minFrequency", [] {})
  , maxFrequency_(*this, "maxFrequency", [] {})
  , backgroundScaling_(*this, "backgroundScaling", [] {})
  , whistleScaling_(*this, "whistleScaling", [] {})
  , numberOfBands_(*this, "numberOfBands", [] {})
  , minWhistleCount_(*this, "minWhistleCount", [] {})
  , channel_(*this, "channel", [] {})
  , fft_(fftBufferSize_)
  , lastTimeWhistleHeard_()
  , foundWhistlesBuffer_(foundWhistlesBufferSize_, false)
{
}

void WhistleDetection::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  if (rawGameControllerState_->gameState != GameState::SET)
  {
    return;
  }
  if (recordData_->samples[channel_()].empty())
  {
    return;
  }

  for (auto& sample : recordData_->samples[channel_()])
  {
    fftBuffer_.push_back(sample);
    if (fftBuffer_.size() == fftBufferSize_)
    {
      // check current fft buffer for whistle
      foundWhistlesBuffer_.push_back(fftBufferContainsWhistle());
      // count the number of found whistles in the whistle buffer
      const unsigned int whistleCount =
          std::accumulate(foundWhistlesBuffer_.begin(), foundWhistlesBuffer_.end(), 0);
      // a whistle is reported if the whistle buffer contains at least a certain number of found
      // whistles
      if (whistleCount >= minWhistleCount_())
      {
        lastTimeWhistleHeard_ = cycleInfo_->startTime;
      }
      fftBuffer_.clear();
      break;
    }
  }

  whistleData_->lastTimeWhistleHeard = lastTimeWhistleHeard_;
}

bool WhistleDetection::fftBufferContainsWhistle()
{
  // apply Hann window to reduce spectral leakage
  for (unsigned int i = 0; i < fftBufferSize_; i++)
  {
    fftBuffer_[i] *=
        std::pow(std::sin(static_cast<float>(M_PI) * static_cast<float>(i) / fftBufferSize_), 2.0f);
  }
  // perform the fft
  auto freqData = fft_.fft(fftBuffer_);

  // the indices corresponding to the whistle band are computed by dividing by the frequency
  // resolution
  double freqResolution = AudioInterface::samplingRate / fftBufferSize_;
  unsigned int minFreqIndex = ceil(minFrequency_() / freqResolution);
  unsigned int maxFreqIndex = ceil(maxFrequency_() / freqResolution);

  if (maxFreqIndex > fftBufferSize_)
  {
    throw std::runtime_error(
        "WhistleDetection: maxFrequency can not be higher than nyquist frequency.");
  }

  // the absolute values of the comlpex spectrum, the mean and the standard deviation
  std::vector<float> absFreqData(freqData.size());
  for (unsigned int i = 0; i < freqData.size(); i++)
  {
    absFreqData[i] = std::abs(freqData[i]);
  }
  debug().update(mount_ + ".absFreqData", absFreqData);
  const float mean = Statistics::mean(absFreqData);
  const float standardDeviation = Statistics::standardDeviation(absFreqData, mean);

  // the spectrum is divided into several bands. for each band, the mean is compared to the
  // background threshold to find the whistle band
  const float backgroundThreshold = mean + backgroundScaling_() * standardDeviation;
  const unsigned int bandSize = ceil((maxFreqIndex - minFreqIndex) / numberOfBands_());

  // find the start of the the whistle band
  for (unsigned int i = 0; i < numberOfBands_(); i++)
  {
    const std::vector<float>::const_iterator bandStart = absFreqData.begin() + minFreqIndex;
    const std::vector<float>::const_iterator bandEnd =
        absFreqData.begin() + minFreqIndex + bandSize;
    const float bandMean = Statistics::mean(std::vector<float>(bandStart, bandEnd));
    if (bandMean < backgroundThreshold)
    {
      minFreqIndex += bandSize;
    }
    else
    {
      break;
    }
  }

  // find the end of the whistle band
  for (unsigned int i = 0; i < numberOfBands_(); i++)
  {
    const std::vector<float>::const_iterator bandStart =
        absFreqData.begin() + maxFreqIndex - bandSize;
    const std::vector<float>::const_iterator bandEnd = absFreqData.begin() + maxFreqIndex;
    const float bandMean = Statistics::mean(std::vector<float>(bandStart, bandEnd));
    if (bandMean < backgroundThreshold)
    {
      maxFreqIndex -= bandSize;
    }
    else
    {
      break;
    }
  }

  Uni::Value freqIndices = Uni::Value(Uni::ValueType::OBJECT);
  freqIndices["minFreqIndex"] << minFreqIndex;
  freqIndices["maxFreqIndex"] << maxFreqIndex;
  debug().update(mount_ + ".freqIndices", freqIndices);

  // a whistle is found in the buffer if the mean of the whistle band is significantly larger than a
  // threshold
  if (minFreqIndex < maxFreqIndex)
  {
    const std::vector<float>::const_iterator bandStart = absFreqData.begin() + minFreqIndex;
    const std::vector<float>::const_iterator bandEnd = absFreqData.begin() + maxFreqIndex;
    const float whistleMean = Statistics::mean(std::vector<float>(bandStart, bandEnd));
    const float whistleThreshold = mean + whistleScaling_() * standardDeviation;
    if (whistleMean > whistleThreshold)
    {
      return true;
    }
    else
    {
      return false;
    }
  }
  else
  {
    return false;
  }
}
