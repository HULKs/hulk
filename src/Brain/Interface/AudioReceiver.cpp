#include "Brain/Interface/AudioReceiver.hpp"


AudioReceiver::AudioReceiver(const ModuleManagerInterface& manager)
  : Module(manager)
  , audioInterface_(robotInterface().getAudio())
  , recordData_(*this)
{
  audioInterface_.startCapture();
  for (unsigned int channel = 0; channel < AudioInterface::numChannels; channel++)
  {
    subsampledData_[channel].reserve(AudioInterface::captureSamplingRate);
  }
}

AudioReceiver::~AudioReceiver()
{
  audioInterface_.stopCapture();
}

void AudioReceiver::cycle()
{
  if (!audioInterface_.getAudioProperties().recordingSupported)
  {
    return;
  }
  // read audio data from all 4 microphones into circular buffer
  audioInterface_.readAudioData(recordData_->samples, recordData_->cycleStartIterators);
  // only plot a part of the buffer to avoid high amounts of network data
  const float plotFraction = 0.75;
  for (unsigned int channel = 0; channel < AudioInterface::numChannels; channel++)
  {
    const size_t bufferSize = recordData_->samples[channel].size();
    if (bufferSize)
    {
      subsampledData_[channel].clear();
      for (size_t i = plotFraction * bufferSize; i < bufferSize; i += 5)
      {
        subsampledData_[channel].push_back(recordData_->samples[channel][i]);
      }
      debug().update(mount_ + ".audioSamples_" + audioInterface_.microphoneNames[channel],
                     subsampledData_[channel]);
    }
  }
  recordData_->valid = true;
}
