#include "Brain/Interface/AudioPlayer.hpp"
#include "Framework/DebugDatabase.hpp"
#include "Framework/Log/Log.hpp"
#include "Framework/ModuleManagerInterface.hpp"
#include <opusfile.h>


AudioPlayer::AudioPlayer(const ModuleManagerInterface& manager)
  : Module{manager}
  , manager_{manager}
  , audioInterface_{robotInterface().getAudio()}
  , cycleInfo_{*this}
  , playbackCooldownTime_{*this, "playbackCooldownTime", [] {}}
  , samePlayerNumberCooldownTime_{*this, "samePlayerNumberCooldownTime", [] {}}
{
  audioInterface_.startPlayback();

  const auto fileRoot = robotInterface().getFileRoot();

  mappedSounds_.emplace(AudioSounds::OUCH, AudioFile(fileRoot + "sounds/ouch.ogg",
                                                     playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(AudioSounds::DONK, AudioFile(fileRoot + "sounds/donk.ogg",
                                                     playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(AudioSounds::BALL, AudioFile(fileRoot + "sounds/ball.ogg",
                                                     playbackCooldownTime_(), audioInterface_));

  // Directions
  mappedSounds_.emplace(AudioSounds::LEFT, AudioFile(fileRoot + "sounds/left.ogg",
                                                     playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(AudioSounds::RIGHT, AudioFile(fileRoot + "sounds/right.ogg",
                                                      playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(AudioSounds::FRONT, AudioFile(fileRoot + "sounds/front.ogg",
                                                      playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::FRONT_LEFT,
      AudioFile(fileRoot + "sounds/frontLeft.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::FRONT_RIGHT,
      AudioFile(fileRoot + "sounds/frontRight.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(AudioSounds::REAR, AudioFile(fileRoot + "sounds/rear.ogg",
                                                     playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::REAR_LEFT,
      AudioFile(fileRoot + "sounds/rearLeft.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::REAR_RIGHT,
      AudioFile(fileRoot + "sounds/rearRight.ogg", playbackCooldownTime_(), audioInterface_));

  // MISC
  mappedSounds_.emplace(
      AudioSounds::CAMERA_RESET,
      AudioFile(fileRoot + "sounds/cameraReset.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::CENTER_CIRCLE,
      AudioFile(fileRoot + "sounds/centerCircle.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::LOLA_DESYNC,
      AudioFile(fileRoot + "sounds/lolaDesync.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::PENALTY_AREA,
      AudioFile(fileRoot + "sounds/penaltyArea.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::PENALTY_SPOT,
      AudioFile(fileRoot + "sounds/penaltySpot.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(AudioSounds::SQUAT, AudioFile(fileRoot + "sounds/squat.ogg",
                                                      playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::T_JUNCTION,
      AudioFile(fileRoot + "sounds/tJunction.ogg", playbackCooldownTime_(), audioInterface_));

  // Playing roles
  mappedSounds_.emplace(
      AudioSounds::PLAYING_ROLE_STRIKER,
      AudioFile(fileRoot + "sounds/striker.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::PLAYING_ROLE_KEEPER,
      AudioFile(fileRoot + "sounds/keeper.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(AudioSounds::PLAYING_ROLE_REPLACEMENT_KEEPER,
                        AudioFile(fileRoot + "sounds/replacementKeeper.ogg",
                                  playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::PLAYING_ROLE_SUPPORTER,
      AudioFile(fileRoot + "sounds/supporter.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::PLAYING_ROLE_BISHOP,
      AudioFile(fileRoot + "sounds/bishop.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::PLAYING_ROLE_DEFENDER,
      AudioFile(fileRoot + "sounds/defender.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::PLAYING_ROLE_DEFENDER_LEFT,
      AudioFile(fileRoot + "sounds/defenderLeft.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::PLAYING_ROLE_DEFENDER_RIGHT,
      AudioFile(fileRoot + "sounds/defenderRight.ogg", playbackCooldownTime_(), audioInterface_));

  // MISC
  mappedSounds_.emplace(AudioSounds::FALSE_POSITIVE_DETECTED,
                        AudioFile(fileRoot + "sounds/falsePositiveDetected.ogg",
                                  playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::FALSE_POSITIVE,
      AudioFile(fileRoot + "sounds/falsePositive.ogg", playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(AudioSounds::WEEEEE, AudioFile(fileRoot + "sounds/weeeee.ogg",
                                                       playbackCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(AudioSounds::DRIFT, AudioFile(fileRoot + "sounds/drift.ogg",
                                                      playbackCooldownTime_(), audioInterface_));

  // initialize "samePlayerNumber" audio files. Assuming we have naos 21 to 36
  for (unsigned int nao = 1; nao < 16; nao++)
  {
    const int naoBase = 20;
    const std::string fileName =
        fileRoot + "sounds/sameNumbertuhhNao" + std::to_string(nao + naoBase) + ".ogg";
    Log<M_BRAIN>(LogLevel::DEBUG) << "Creating audio file \"" << fileName << "\" with id "
                                  << static_cast<unsigned int>(
                                         AudioSounds::SAME_PLAYER_NUMBER_MIN) +
                                         nao;
    mappedSounds_.emplace(static_cast<AudioSounds>(
                              static_cast<unsigned int>(AudioSounds::SAME_PLAYER_NUMBER_MIN) + nao),
                          AudioFile(fileName, samePlayerNumberCooldownTime_(), audioInterface_));
  }

  mappedSounds_.emplace(AudioSounds::SAME_PLAYER_NUMBER_GENERAL_ETH,
                        AudioFile(fileRoot + "sounds/sameNumberUnknownHULKDeviceETH.ogg",
                                  samePlayerNumberCooldownTime_(), audioInterface_));
  mappedSounds_.emplace(AudioSounds::SAME_PLAYER_NUMBER_GENERAL_WIFI,
                        AudioFile(fileRoot + "sounds/sameNumberUnknownHULKDeviceWIFI.ogg",
                                  samePlayerNumberCooldownTime_(), audioInterface_));

  mappedSounds_.emplace(
      AudioSounds::USB_STICK_MISSING,
      AudioFile(fileRoot + "sounds/usbStickMissing.ogg", playbackCooldownTime_(), audioInterface_));
}

AudioPlayer::~AudioPlayer()
{
  audioInterface_.stopPlayback();
}

void AudioPlayer::cycle()
{
  auto databases = manager_.getDebugDatabases();
  for (const auto* database : databases)
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
    if (!aFile.play(*cycleInfo_))
    {
      // Putting the requested sound back into the queue because it
      // could not be played this time around.
      requestedSounds_.insert(aSound);
    }
  }
  else
  {
    Log<M_BRAIN>(LogLevel::ERROR) << "Requested sound not found: " << static_cast<int>(aSound);
    assert(false);
  }
}

AudioFile::AudioFile(const std::string& filename, Clock::duration playbackCooldownTime,
                     AudioInterface& audioInterface)
  : audioInterface_(audioInterface)
  , playbackCooldownTime_(playbackCooldownTime)
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
  while ((readFrames =
              op_read_float(opusfile, samples_.data() + totalReadFrames,
                            static_cast<int>(samples_.size()) - totalReadFrames, nullptr)) > 0)
  {
    totalReadFrames += readFrames;
  }

  if (totalPcmFrames != totalReadFrames)
  {
    Log<M_BRAIN>(LogLevel::ERROR) << "Unable to load audio file: " << filename
                                  << ": totalPcmFrames != totalReadFrames";
    assert(false);
  }
}

bool AudioFile::play(const CycleInfo& cycleInfo)
{
  if (!audioInterface_.getAudioProperties().playbackSupported)
  {
    return false;
  }
  if (!audioInterface_.isPlaybackFinished() ||
      cycleInfo.getAbsoluteTimeDifference(lastTimePlayed_) < playbackCooldownTime_)
  {
    return false;
  }
  lastTimePlayed_ = cycleInfo.startTime;
  audioInterface_.playbackAudioData(samples_);
  return true;
}
