#pragma once

#include "Hardware/AudioInterface.hpp"

class ReplayAudio : public AudioInterface
{
public:
  ReplayAudio();

  ~ReplayAudio();
  /**
   * @see AudioInterface
   */
  AudioProperties getAudioProperties() override;
  /**
   * @see AudioInterface
   */
  void readAudioData(std::array<SampleRingBuffer, AudioInterface::numChannels>& /*recordData*/,
                     std::array<SampleRingBufferIt,
                                AudioInterface::numChannels>& /*cycleStartIterators*/) override;

  /**
   * @see AudioInterface
   */
  void playbackAudioData(const Samples& samples) override;
  void startPlayback() override;
  void stopPlayback() override;
  void startCapture() override;
  void stopCapture() override;
  bool isPlaybackFinished() override;
  void clearPlaybackBuffer() override;

private:
  /// The audio device properties of this platform
  AudioProperties properties_;
};
