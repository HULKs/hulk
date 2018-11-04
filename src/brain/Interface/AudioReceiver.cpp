#include "AudioReceiver.hpp"


AudioReceiver::AudioReceiver(const ModuleManagerInterface& manager)
  : Module(manager)
  , audioInterface_(robotInterface().getAudio())
  , recordData_(*this)
{
  audioInterface_.startCapture();
}

AudioReceiver::~AudioReceiver()
{
  audioInterface_.stopCapture();
}

void AudioReceiver::cycle()
{
  audioInterface_.readAudioData(recordData_->samples);
}
