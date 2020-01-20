#pragma once

#include <vector>

#include <Framework/DataType.hpp>
#include <Tools/Time.hpp>
#include <Hardware/AudioInterface.hpp>

template <unsigned int numChannels=1>
class AudioData : public DataType<AudioData<numChannels>> {
public:
  /// the name of this DataType
  DataTypeName name = "AudioData";
  /// a sequence of samples that should be played back or have been recorded
  std::array<Samples, numChannels> samples;
  /// the timestamp at which the first sample has been recorded or shall be played back
  TimePoint timestamp;
  /**
   * @brief reset clears the sequence of samples
   */
  void reset() override
  {
    for (unsigned int channel = 0; channel < numChannels; channel++)
    {
      samples[channel].clear();
    }
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["samples"] << samples;
    value["timestamp"] << timestamp;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["samples"] >> samples;
    value["timestamp"] >> timestamp;
  }
};

template <unsigned int numChannels=1>
class RecordData : public DataType<RecordData<numChannels>, AudioData<numChannels>> {
public:
  /// the name of this DataType
  DataTypeName name = "RecordData";
};

template <unsigned int numChannels=1>
class PlaybackData : public DataType<PlaybackData<numChannels>, AudioData<numChannels>> {
public:
  /// the name of this DataType
  DataTypeName name = "PlaybackData";
};
