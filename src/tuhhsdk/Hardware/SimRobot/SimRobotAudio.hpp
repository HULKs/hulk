#pragma once

#include "Hardware/AudioInterface.hpp"

class SimRobotAudio : public AudioInterface
{
public:
  /**
   * @see AudioInterface
   */
  void readAudioData(Samples& samples);
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
};
