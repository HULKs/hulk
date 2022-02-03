#pragma once

#include "Hardware/AudioInterface.hpp"

class SimRobotNoAudio : public AudioInterface
{
public:
  SimRobotNoAudio();

  ~SimRobotNoAudio();
  /**
   * @see AudioInterface
   */
  AudioProperties getAudioProperties() override;
  /**
   * @see AudioInterface
   */
  void
  readAudioData(std::array<SampleRingBuffer, AudioInterface::numChannels>& /*recordData*/,
                std::array<SampleRingBufferIt, AudioInterface::numChannels>& /*cycleStartIterators*/
                ) override;

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
  /// The audio device properties of this platform
  AudioProperties properties_{};
};
