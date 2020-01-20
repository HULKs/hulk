#pragma once

#include <deque>
#include <mutex>

#include <Tools/Time.hpp>

typedef std::vector<float> Samples;

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

  /// Utilize all four microphones
  static const constexpr unsigned int numChannels =
      static_cast<unsigned int>(Microphone::MICROPHONE_MAX);

  /// the sampling rate for both playback and record. In samples per second, i.e. Hz
  static constexpr unsigned int samplingRate = 48000; // 44100;

  /// The microphone names to use for debug outputs
  const char* microphoneNames[numChannels] = {"rearLeft", "rearRight", "frontLeft", "frontRight"};

  /**
   * @brief ~AudioInterface a virtual destructor for polymorphism
   */
  virtual ~AudioInterface() {}

  /**
   * @brief readAudioData gets all data currently buffered from the microphones of the NAO
   * @param audio_data is filled with the new audio data
   */
  virtual void readAudioData(Samples& audio_data, const Microphone microphone) = 0;
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
