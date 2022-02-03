#include "WebotsAudio.hpp"

AudioInterface::AudioProperties WebotsAudio::getAudioProperties()
{
  return {false, false};
}

void WebotsAudio::readAudioData(
    [[maybe_unused]] std::array<SampleRingBuffer, numChannels>& recordSamples,
    [[maybe_unused]] std::array<SampleRingBufferIt, numChannels>& cycleStartIterators)
{
}

void WebotsAudio::playbackAudioData([[maybe_unused]] const Samples& audioData) {}

void WebotsAudio::startPlayback() {}

void WebotsAudio::stopPlayback() {}

void WebotsAudio::startCapture() {}

void WebotsAudio::stopCapture() {}

bool WebotsAudio::isPlaybackFinished()
{
  return false;
}

void WebotsAudio::clearPlaybackBuffer() {}
