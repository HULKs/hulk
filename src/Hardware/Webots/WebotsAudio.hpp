#pragma once

#include "Hardware/AudioInterface.hpp"

class WebotsAudio : public AudioInterface
{
public:
  AudioProperties getAudioProperties() override;
  void readAudioData(std::array<SampleRingBuffer, numChannels>& recordSamples,
                     std::array<SampleRingBufferIt, numChannels>& cycleStartIterators) override;
  void playbackAudioData(const Samples& audioData) override;
  void startPlayback() override;
  void stopPlayback() override;
  void startCapture() override;
  void stopCapture() override;
  bool isPlaybackFinished() override;
  void clearPlaybackBuffer() override;
};
