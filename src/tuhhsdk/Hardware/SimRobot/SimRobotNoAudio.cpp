#include "SimRobotNoAudio.hpp"
#include "print.h"

SimRobotNoAudio::SimRobotNoAudio() {}

void SimRobotNoAudio::startCapture() {}

void SimRobotNoAudio::stopCapture() {}

void SimRobotNoAudio::startPlayback() {}

void SimRobotNoAudio::stopPlayback() {}

SimRobotNoAudio::~SimRobotNoAudio() {}

void SimRobotNoAudio::readAudioData(Samples& /*audio_data*/,
                                    const AudioInterface::Microphone /*microphone*/)
{
}

void SimRobotNoAudio::playbackAudioData(const Samples& /*samples*/) {}

bool SimRobotNoAudio::isPlaybackFinished()
{
  return true;
}

void SimRobotNoAudio::clearPlaybackBuffer() {}
