#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Hardware/AudioInterface.hpp"
#include "Hardware/Clock.hpp"

template <unsigned int numChannels = 1>
class AudioData : public DataType<AudioData<numChannels>>
{
public:
  /// the name of this DataType
  DataTypeName name__{"AudioData"};
  /// size of ring buffer (1.5seconds)
  static constexpr uint BUFFERSIZE = 66150;
  /// a sequence of samples that should be played back or have been recorded
  std::array<SampleRingBuffer, numChannels> samples{
      {SampleRingBuffer(BUFFERSIZE), SampleRingBuffer(BUFFERSIZE), SampleRingBuffer(BUFFERSIZE),
       SampleRingBuffer(BUFFERSIZE)}};
  /// the timestamp at which the first sample has been recorded or shall be played back
  Clock::time_point timestamp;
  /// iterators that point to the first sample of the current cycle (for each channel)
  std::array<SampleRingBufferIt, numChannels> cycleStartIterators;

  void reset() override
  {
    // nothing
  }

  void toValue(Uni::Value& value) const override
  {
    std::array<Samples, numChannels> vector;
    for (uint channel = 0; channel < numChannels; channel++)
    {
      vector[channel].assign(samples[channel].begin(), samples[channel].end());
    }
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["samples"] << vector;
    value["timestamp"] << timestamp;
  }

  void fromValue(const Uni::Value& value) override
  {
    std::array<Samples, numChannels> vector;
    value["samples"] >> vector;
    value["timestamp"] >> timestamp;
    for (uint channel = 0; channel < numChannels; channel++)
    {
      samples[channel].clear();
      samples[channel].insert(samples[channel].end(), vector[channel].begin(),
                              vector[channel].end());
    }
  }
};

template <unsigned int numChannels = 1>
class RecordData : public DataType<RecordData<numChannels>, AudioData<numChannels>>
{
public:
  /// the name of this DataType
  DataTypeName name__{"RecordData"};
  /// whether the data of this DataType is valid or not.
  bool valid{false};

  void reset() override
  {
    valid = false;
  }
};

template <unsigned int numChannels = 1>
class PlaybackData : public DataType<PlaybackData<numChannels>, AudioData<numChannels>>
{
public:
  /// the name of this DataType
  DataTypeName name__{"PlaybackData"};
};
