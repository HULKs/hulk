#include "Hardware/SimRobot/SimRobotNoAudio.hpp"
#include "Framework/Log/Log.hpp"

SimRobotNoAudio::SimRobotNoAudio()
{
  properties_.playbackSupported = false;
  properties_.recordingSupported = false;
}

void SimRobotNoAudio::startCapture() {}

void SimRobotNoAudio::stopCapture() {}

void SimRobotNoAudio::startPlayback() {}

void SimRobotNoAudio::stopPlayback() {}

SimRobotNoAudio::~SimRobotNoAudio() {}

AudioInterface::AudioProperties SimRobotNoAudio::getAudioProperties()
{
  return properties_;
}

void SimRobotNoAudio::readAudioData(
    std::array<SampleRingBuffer, AudioInterface::numChannels>& /*recordData*/,
    std::array<SampleRingBufferIt, AudioInterface::numChannels>& /*cycleStartIterators*/)
{
  assert(properties_.recordingSupported);
}

void SimRobotNoAudio::playbackAudioData(const Samples& /*samples*/)
{
  assert(properties_.playbackSupported);
}

bool SimRobotNoAudio::isPlaybackFinished()
{
  return true;
}

void SimRobotNoAudio::clearPlaybackBuffer() {}
