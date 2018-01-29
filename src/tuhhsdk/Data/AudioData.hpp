#pragma once

#include <vector>

#include <Framework/DataType.hpp>
#include <Tools/Time.hpp>
#include <Hardware/AudioInterface.hpp>

class AudioData : public DataType<AudioData> {
public:
  /// a sequence of samples that should be played back or have been recorded
  Samples samples;
  /// the timestamp at which the first sample has been recorded or shall be played back
  TimePoint timestamp;
  /**
   * @brief reset clears the sequence of samples
   */
  void reset()
  {
    samples.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["samples"] << samples;
    value["timestamp"] << timestamp;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["samples"] >> samples;
    value["timestamp"] >> timestamp;
  }
};

class RecordData : public DataType<RecordData, AudioData> {
};

class PlaybackData : public DataType<PlaybackData, AudioData> {
};
