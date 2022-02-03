#pragma once

#include "Data/AudioData.hpp"
#include "Data/CycleInfo.hpp"
#include "Framework/Module.hpp"
#include "Hardware/AudioInterface.hpp"

class Brain;

/**
 * @brief Class to handle playing of sound files
 */
class AudioFile
{
public:
  /**
   * @brief AudioFile constructor
   * @param path to the audio file
   * @param playBackCooldownTime the default minimum time span before playing this file again
   * @param reference to the audio interface to play the file
   */
  AudioFile(const std::string& filename, Clock::duration playBackCooldownTime,
            AudioInterface& audioInterface);

  /**
   * @brief AudioFile move constructor
   * @param other the audio file to move
   */
  AudioFile(AudioFile&& other) = default;

  // Delete copy constructor and assignment operator. They may not be used for the AudioFile
  AudioFile(const AudioFile& other) = delete;
  AudioFile& operator=(const AudioFile&) = delete;

  /**
   * @brief play is used to play the sound. It checks if the file is ready to be played.
   *
   * Default playback cooldown is used
   *
   * @return whether the sound could be played.
   */
  bool play(const CycleInfo& cycleInfo);

private:
  /// a reference to the wrapper for the (possibly hardware specific) audio interface
  AudioInterface& audioInterface_;
  /// the timepoint this sound was played last
  Clock::time_point lastTimePlayed_;

  /// the default cool down time to use
  Clock::duration playbackCooldownTime_;
  /// a object to hold the sound samples
  Samples samples_;
};

class AudioPlayer : public Module<AudioPlayer, Brain>
{
public:
  ModuleName name__{"AudioPlayer"};
  explicit AudioPlayer(const ModuleManagerInterface& manager);
  ~AudioPlayer() override;
  void cycle() override;

private:
  /// the brain moduleManagerInterface
  const ModuleManagerInterface& manager_;
  /// a reference to the wrapper for the (possibly hardware specific) audio interface
  AudioInterface& audioInterface_;

  const Dependency<CycleInfo> cycleInfo_;

  /// the time that must be wait until the next sound can be played
  Parameter<Clock::duration> playbackCooldownTime_;
  /// the time that must be wait until the next "same player number" sound can be played
  Parameter<Clock::duration> samePlayerNumberCooldownTime_;

  /// a set for all sounds that are currently requested
  std::set<AudioSounds, std::greater<>> requestedSounds_;

  /// a map for holding AudioFile(s), mapped by AudioSounds
  std::map<AudioSounds, AudioFile> mappedSounds_;
};
