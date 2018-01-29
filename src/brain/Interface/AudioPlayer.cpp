//
// Created by Finn Poppinga on 04.05.16.
//

#include "AudioPlayer.hpp"


AudioPlayer::AudioPlayer(const ModuleManagerInterface& manager)
  : Module(manager, "AudioPlayer")
  , audioInterface_(robotInterface().getAudio())
  , playbackData_(*this)
{
  audioInterface_.startPlayback();
}

AudioPlayer::~AudioPlayer()
{
  audioInterface_.stopPlayback();
}

void AudioPlayer::cycle()
{
  if (!playbackData_->samples.empty())
  {
    audioInterface_.playbackAudioData(playbackData_->samples);
  }
}
