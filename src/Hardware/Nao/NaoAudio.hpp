#pragma once

#include <alsa/asoundlib.h>
#include <atomic>
#include <condition_variable>
#include <thread>

#include "Hardware/AudioInterface.hpp"

class NaoAudio : public AudioInterface
{
public:
  NaoAudio();

  ~NaoAudio();
  /**
   * @see AudioInterface
   */
  AudioProperties getAudioProperties() override;
  /**
   * @see AudioInterface
   */
  void readAudioData(std::array<SampleRingBuffer, numChannels>& recordData,
                     std::array<SampleRingBufferIt, numChannels>& cycleStartIterators) override;
  /**
   * @see AudioInterface
   */
  void playbackAudioData(const Samples& samples) override;
  /**
   * @see AudioInterface
   */
  void startPlayback() override;
  /**
   * @see AudioInterface
   */
  void stopPlayback() override;

  /**
   * @see AudioInterface
   */
  void startCapture() override;
  /**
   * @see AudioInterface
   */
  void stopCapture() override;
  /**
   * @see AudioInterface
   */
  bool isPlaybackFinished() override;
  /**
   * @see AudioInterface
   */
  void clearPlaybackBuffer() override;

private:
  /**
   * @brief initialization of pcm parameters for audio capture
   */
  void initCapture();
  /**
   * @brief initialization of pcm parameters for audio playback
   */
  void initPlayback();

  /// sample rate for playback
  unsigned int captureSampleRate_;

  /// sample rate for capture
  unsigned int playbackSampleRate_;

  /// thread for receiving microphone data
  std::thread captureThread_;

  /// run/stop capture thread
  std::atomic_bool runCaptureThread_;

  /// pcm handle for audio capture
  snd_pcm_t* captureHandle_;

  /// thread for speaker output
  std::thread playbackThread_;

  /// run/stop playback thread
  std::atomic_bool runPlaybackThread_;

  /// condition variable to signal when there is content for the playback buffer
  std::condition_variable playbackCondition_;

  /// pcm handle for audio playback
  snd_pcm_t* playbackHandle_;

  /// The buffer size to be sent/consumed from the sound devices per Buffer
  static constexpr unsigned int framesPerBuffer = 512;

  /// mutex that locks input buffer while reading in microphone data
  std::mutex inBufferLock_;

  /// This buffer stores the recorded samples
  AudioBuffer<float> inBuffer_[numChannels];
  /// This buffer stores the samples to play back
  AudioBuffer<float> outBuffer_;

  /// The audio device properties of this platform
  AudioProperties properties_;
};
