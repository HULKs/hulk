#pragma once

#include <portaudio.h>

#include <Hardware/AudioInterface.hpp>
#include <Tools/Var/SpscQueue.hpp>

class NaoAudio : public AudioInterface
{
public:
  NaoAudio();

  ~NaoAudio();
  /**
   * @see AudioInterface
   */
  void readAudioData(Samples& samples, const Microphone microphone);
  /**
   * @see AudioInterface
   */
  void playbackAudioData(const Samples& samples);
  /**
   * @see AudioInterface
   */
  void startPlayback();
  /**
   * @see AudioInterface
   */
  void stopPlayback();

  /**
   * @see AudioInterface
   */
  void startCapture();
  /**
   * @see AudioInterface
   */
  void stopCapture();
  /**
   * @see AudioInterface
   */
  bool isPlaybackFinished();
  /**
   * @see AudioInterface
   */
  void clearPlaybackBuffer();

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
  /// This buffer stores the recorded samples
  AudioBuffer<float> inBuffer_[numChannels];
  /// This buffer stores the samples to play back
  AudioBuffer<float> outBuffer_;
  /// PortAudio stream for capture
  PaStream* inStream_;
  /// PortAudio stream for playback
  PaStream* outStream_;
};
