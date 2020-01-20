#pragma once

#include <Hardware/AudioInterface.hpp>

class ReplayAudio : public AudioInterface
{
public:
  ReplayAudio();

  ~ReplayAudio();
  /**
   * @see AudioInterface
   */
  void readAudioData(Samples& samples, const AudioInterface::Microphone) override;
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
};
