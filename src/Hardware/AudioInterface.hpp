#pragma once

#include <deque>
#include <mutex>

#include "Hardware/Clock.hpp"
#include <boost/circular_buffer.hpp>

typedef std::vector<float> Samples;
typedef boost::circular_buffer<float> SampleRingBuffer;
typedef SampleRingBuffer::const_iterator SampleRingBufferIt;

template <typename T>
struct AudioBuffer
{
  /// the actual buffer
  std::deque<T> buffer;
  /// lock to synchronize access to the buffer
  std::mutex lock;
};

class AudioInterface
{
public:
  /// The microphone locations from the NAOs perspective and their channel indices
  enum class Microphone
  {
    REAR_LEFT,
    REAR_RIGHT,
    FRONT_LEFT,
    FRONT_RIGHT,
    MICROPHONE_MAX
  };

  struct AudioProperties
  {
    bool playbackSupported;
    bool recordingSupported;
  };

  /// Utilize all four microphones
  static const constexpr unsigned int numChannels =
      static_cast<unsigned int>(Microphone::MICROPHONE_MAX);

  /// the sampling rate for record. In samples per second, i.e. Hz
  static constexpr unsigned int captureSamplingRate = 44100;

  /// the sampling rate for playback. In samples per second, i.e. Hz
  static constexpr unsigned int playbackSamplingRate = 48000;

  /// The microphone names to use for debug outputs
  const char* microphoneNames[numChannels] = {"rearLeft", "rearRight", "frontLeft", "frontRight"};

  /**
   * @brief ~AudioInterface a virtual destructor for polymorphism
   */
  virtual ~AudioInterface() {}

  /**
   * @brief Returns the audio properties of the platform
   * @return the audio properties
   */
  virtual AudioProperties getAudioProperties() = 0;

  /**
   * @brief readAudioData gets all data currently buffered from the microphones of the NAO
   * @param recordSamples is filled with the new audio data
   * @param cycleStartIterators iterators that point to the first sample of the current cycle (for
   * each channel)
   * @pre properties.recordingSupported()
   */
  virtual void readAudioData(std::array<SampleRingBuffer, numChannels>& recordSamples,
                             std::array<SampleRingBufferIt, numChannels>& cycleStartIterators) = 0;
  /**
   * @brief playAudioData plays back the samples provided as a parameter
   * @param audio_data the samples (stereo interlaced) to send to the speakers of the NAO
   */
  virtual void playbackAudioData(const Samples& audio_data) = 0;
  /**
   * @brief startPlayback starts streaming of samples for playback. Needs to be called before
   * anything will be played back.
   */
  virtual void startPlayback() = 0;
  /**
   * @brief stopPlayback stops playback streaming
   */
  virtual void stopPlayback() = 0;
  /**
   * @brief startCapture starts streaming of samples for capturing. Needs to be called before
   * anything will be recorded.
   */
  virtual void startCapture() = 0;
  /**
   * @brief stopCapture stops capture streaming
   */
  virtual void stopCapture() = 0;
  /**
   * @brief isPlaybackFinished whether the playback is finished
   */
  virtual bool isPlaybackFinished() = 0;
  /**
   * @brief clearPlaybackBuffer clears the playback buffer
   */
  virtual void clearPlaybackBuffer() = 0;
};
