#pragma once

#include <portaudio.h>

#include "Hardware/AudioInterface.hpp"
#include "Tools/Var/SpscQueue.hpp"

class SimRobotPortAudio : public AudioInterface
{
public:
  SimRobotPortAudio();

  ~SimRobotPortAudio();
  /**
   * @see AudioInterface
   */
  AudioProperties getAudioProperties() override;
  /**
   * @see AudioInterface
   */
  void readAudioData(
      std::array<SampleRingBuffer, AudioInterface::numChannels>& recordData,
      std::array<SampleRingBufferIt, AudioInterface::numChannels>& cycleStartIterators) override;
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
   * @brief The PortAudio callback called when playing back samples.
   * @param inputBuffer ignored
   * @param outputBuffer is the buffer that will be actually sent to the audio device
   * @param framesPerBuffer the number of frames in a buffer
   * @param timeInfo ignored
   * @param statusFlags ignored
   * @param userData is used to pass a this pointer to this callback.
   */
  static int playbackCallback(const void* inputBuffer, void* outputBuffer,
                              unsigned long framesPerBuffer,
                              const PaStreamCallbackTimeInfo* timeInfo, PaStreamFlags statusFlags,
                              void* userData);
  /**
   * @brief The PortAudio callback called, when recording samples.
   * @param inputBuffer is the buffer, that contains the samples recorded from the audio device
   * @param outputBuffer ignored
   * @param framesPerBuffer the number of frames in a buffer
   * @param timeInfo ignored
   * @param statusFlags ignored
   * @param userData is used to pass a this pointer to this callback.
   */
  static int recordCallback(const void* inputBuffer, void* outputBuffer,
                            unsigned long framesPerBuffer, const PaStreamCallbackTimeInfo* timeInfo,
                            PaStreamFlags statusFlags, void* userData);
  /**
   * @brief will be called when recording is finished.
   * @param userData is used to pass a this pointer
   */
  static void recordFinishedCallback(void* userData);
  /**
   * @brief will be called when playback is finished.
   * @param userData is used to pass a this pointer
   */
  static void playbackFinishedCallback(void* userData);
  /**
   * @brief Throws a std::runtime_error for PaErrors.
   * @param err the PaError to handle.
   */
  void handlePaErrorCode(PaError err);
  /// The buffer size to be sent/consumed from the sound devices per Buffer
  static constexpr unsigned int framesPerBuffer = 512;
  /// Mutex that locks input buffer while reading in microphone data
  std::mutex inBufferLock_;
  /// This buffer stores the recorded samples
  AudioBuffer<float> inBuffer_[numChannels];
  /// This buffer stores the samples to play back
  AudioBuffer<float> outBuffer_;
  /// PortAudio stream for capture
  PaStream* inStream_;
  /// PortAudio stream for playback
  PaStream* outStream_;

  /// The audio device properties of this platform
  AudioProperties properties_;
};
