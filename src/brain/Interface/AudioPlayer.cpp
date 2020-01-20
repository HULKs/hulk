//
// Created by Finn Poppinga on 04.05.16.
//
#include <opusfile.h>

#include "print.h"

#include "AudioPlayer.hpp"
#include "Framework/DebugDatabase.hpp"
#include "Framework/ModuleManagerInterface.hpp"


AudioPlayer::AudioPlayer(const ModuleManagerInterface& manager)
  : Module(manager)
  , manager_(manager)
  , audioInterface_(robotInterface().getAudio())
  , playbackCooldownTime_(*this, "playbackCooldownTime", [] {})
{
  audioInterface_.startPlayback();

  const auto fileRoot = robotInterface().getFileRoot();

  mappedSounds_.emplace(AudioSounds::OUCH,
                        AudioFile(fileRoot + "sounds/ouch.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::DONK,
                        AudioFile(fileRoot + "sounds/donk.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::BALL,
                        AudioFile(fileRoot + "sounds/ball.ogg", audioInterface_));

  // Directions
  mappedSounds_.emplace(AudioSounds::LEFT,
                        AudioFile(fileRoot + "sounds/left.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::RIGHT,
                        AudioFile(fileRoot + "sounds/right.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::FRONT,
                        AudioFile(fileRoot + "sounds/front.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::FRONT_LEFT,
                        AudioFile(fileRoot + "sounds/frontLeft.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::FRONT_RIGHT,
                        AudioFile(fileRoot + "sounds/frontRight.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::REAR,
                        AudioFile(fileRoot + "sounds/rear.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::REAR_LEFT,
                        AudioFile(fileRoot + "sounds/rearLeft.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::REAR_RIGHT,
                        AudioFile(fileRoot + "sounds/rearRight.ogg", audioInterface_));

  // MISC
  mappedSounds_.emplace(AudioSounds::CAMERA_RESET,
                        AudioFile(fileRoot + "sounds/cameraReset.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::CENTER_CIRCLE,
                        AudioFile(fileRoot + "sounds/centerCircle.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::LOLA_DESYNC,
                        AudioFile(fileRoot + "sounds/lolaDesync.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::PENALTY_AREA,
                        AudioFile(fileRoot + "sounds/penaltyArea.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::PENALTY_SPOT,
                        AudioFile(fileRoot + "sounds/penaltySpot.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::SQUAT,
                        AudioFile(fileRoot + "sounds/squat.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::T_JUNCTION,
                        AudioFile(fileRoot + "sounds/tJunction.ogg", audioInterface_));

  // Playing roles
  mappedSounds_.emplace(AudioSounds::PLAYING_ROLE_STRIKER,
                        AudioFile(fileRoot + "sounds/striker.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::PLAYING_ROLE_KEEPER,
                        AudioFile(fileRoot + "sounds/keeper.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::PLAYING_ROLE_REPLACEMENT_KEEPER,
                        AudioFile(fileRoot + "sounds/replacementKeeper.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::PLAYING_ROLE_SUPPORTER,
                        AudioFile(fileRoot + "sounds/supporter.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::PLAYING_ROLE_BISHOP,
                        AudioFile(fileRoot + "sounds/bishop.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::PLAYING_ROLE_DEFENDER,
                        AudioFile(fileRoot + "sounds/defender.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::PLAYING_ROLE_DEFENDER_LEFT,
                        AudioFile(fileRoot + "sounds/defenderLeft.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::PLAYING_ROLE_DEFENDER_RIGHT,
                        AudioFile(fileRoot + "sounds/defenderRight.ogg", audioInterface_));

  // MISC
  mappedSounds_.emplace(AudioSounds::FALSE_POSITIVE_DETECTED,
                        AudioFile(fileRoot + "sounds/falsePositiveDetected.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::FALSE_POSITIVE,
                        AudioFile(fileRoot + "sounds/falsePositive.ogg", audioInterface_));

  mappedSounds_.emplace(AudioSounds::WEEEEE,
                        AudioFile(fileRoot + "sounds/weeeee.ogg", audioInterface_));
}

AudioPlayer::~AudioPlayer()
{
  audioInterface_.stopPlayback();
}

void AudioPlayer::cycle()
{
  auto databases = manager_.getDebugDatabases();
  for (auto database : databases)
  {
    AudioSounds aSound;
    while (database->popLastRequestedSound(aSound))
    {
      requestedSounds_.insert(aSound);
    }
  }

  if (requestedSounds_.empty())
  {
    return;
  }

  AudioSounds aSound = *requestedSounds_.begin();
  requestedSounds_.erase(aSound);

  auto match = mappedSounds_.find(aSound);
  if (match != mappedSounds_.end())
  {
    auto& aFile = match->second;
    if (!aFile.play(playbackCooldownTime_()))
    {
      // Putting the requested sound back into the queue because it
      // could not be played this time around.
      requestedSounds_.insert(aSound);
    }
  }
  else
  {
    Log(LogLevel::ERROR) << "Requested sound not found: " << static_cast<int>(aSound);
    assert(false);
  }
}

AudioFile::AudioFile(const std::string& filename, AudioInterface& audioInterface)
  : audioInterface_(audioInterface)
{
  int error = 0;
  OggOpusFile* opusfile = op_open_file(filename.c_str(), &error);
  if (opusfile == nullptr)
  {
    throw std::runtime_error("Could not load sound file: \"" + filename + "\", error no " +
                             std::to_string(error));
  }
  auto totalPcmFrames = op_pcm_total(opusfile, -1);

  samples_.resize(totalPcmFrames);
  // Save the number of audio frames that could be read
  int totalReadFrames = 0;
  // Save the number of audio frames read in the current
  // op_read_float iteration
  int readFrames = 0;
  while ((readFrames = op_read_float(opusfile, samples_.data() + totalReadFrames,
                                     samples_.size() - totalReadFrames, nullptr)) > 0)
  {
    totalReadFrames += readFrames;
  }

  if (totalPcmFrames != totalReadFrames)
  {
    Log(LogLevel::ERROR) << "Unable to load audio file: " << filename
                         << ": totalPcmFrames != totalReadFrames";
    assert(false);
  }
}

bool AudioFile::play(const int cooldownTime)
{
  if (!audioInterface_.isPlaybackFinished() ||
      getTimeDiff(lastTimePlayed_, TimePoint::getCurrentTime(), TDT::SECS) < cooldownTime)
  {
    return false;
  }

  lastTimePlayed_ = TimePoint::getCurrentTime();
  audioInterface_.playbackAudioData(samples_);
  return true;
}
