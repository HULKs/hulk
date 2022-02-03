#include "Hardware/Replay/ReplayAudio.hpp"
#include "Framework/Log/Log.hpp"

ReplayAudio::ReplayAudio()
{
  properties_.playbackSupported = false;
  properties_.recordingSupported = false;
}

void ReplayAudio::startCapture() {}

void ReplayAudio::stopCapture() {}

void ReplayAudio::startPlayback() {}

void ReplayAudio::stopPlayback() {}

ReplayAudio::~ReplayAudio() {}

AudioInterface::AudioProperties ReplayAudio::getAudioProperties()
{
  return properties_;
}

void ReplayAudio::readAudioData(
    std::array<SampleRingBuffer, AudioInterface::numChannels>& /*recordData*/,
    std::array<SampleRingBufferIt, AudioInterface::numChannels>& /*cycleStartIterators*/)
{
  assert(properties_.recordingSupported);
}

void ReplayAudio::playbackAudioData(const Samples& /*samples*/)
{
  assert(properties_.playbackSupported);
}

bool ReplayAudio::isPlaybackFinished()
{
  return true;
}

void ReplayAudio::clearPlaybackBuffer() {}
