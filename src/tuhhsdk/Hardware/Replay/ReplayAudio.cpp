#include "ReplayAudio.hpp"
#include "print.h"

ReplayAudio::ReplayAudio() {}

void ReplayAudio::startCapture() {}

void ReplayAudio::stopCapture() {}

void ReplayAudio::startPlayback() {}

void ReplayAudio::stopPlayback() {}

ReplayAudio::~ReplayAudio() {}

void ReplayAudio::readAudioData(Samples& /*audio_data*/,
                                const AudioInterface::Microphone /*microphone*/)
{
}

void ReplayAudio::playbackAudioData(const Samples& /*samples*/) {}

bool ReplayAudio::isPlaybackFinished()
{
  return true;
}

void ReplayAudio::clearPlaybackBuffer() {}
