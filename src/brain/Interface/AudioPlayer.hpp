#pragma once

#include <Data/AudioData.hpp>
#include <Framework/Module.hpp>
#include <Hardware/AudioInterface.hpp>

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
   * @param reference to the audio interface to play the file
   */
  AudioFile(const std::string& filename, AudioInterface& audioInterface);

  /**
   * @brief AudioFile move constructor
   * @param other the audio file to move
   */
  AudioFile(AudioFile&& other) = default;

  // Delete copy constructor and assignment operator. They may not be used for the AudioFile
  AudioFile(const AudioFile& other) = delete;
  AudioFile& operator=(const AudioFile&) = delete;

  /**
   * @brief play is used to play the sound. It checks
   *        if the file is ready to be played.
   * @param cooldownTime the time the module needs to wait
   *        until it can be played the next time.
   * @return whether the sound could be played.
   */
  bool play(int cooldownTime);

private:
  /// a reference to the wrapper for the (possibly hardware specific) audio interface
  AudioInterface& audioInterface_;
  /// the timepoint this sound was played last
  TimePoint lastTimePlayed_;
  /// a object to hold the sound samples
  Samples samples_;
};

class AudioPlayer : public Module<AudioPlayer, Brain>
{
public:
  /// the name of this module
  ModuleName name = "AudioPlayer";
  /**
   * @brief AudioPlayer the constructor initializes the AudioReceiver
   * @param manager a ModuleManagerInterface
   */
  AudioPlayer(const ModuleManagerInterface& manager);
  /**
   * @brief ~AudioPlayer stops audio streaming
   */
  ~AudioPlayer();
  /**
   * @brief cycle transports samples to the hardware audio device
   */
  void cycle() override;

private:
  /// the brain modulemanagerinterface
  const ModuleManagerInterface& manager_;
  /// a reference to the wrapper for the (possibly hardware specific) audio interface
  AudioInterface& audioInterface_;

  /// the time that must be wait until the next sound can be played
  Parameter<int> playbackCooldownTime_;

  /// a set for all sounds that are currently requested
  std::set<AudioSounds, std::greater<>> requestedSounds_;

  /// a map for holding AudioFile(s), mapped by AudioSounds
  std::map<AudioSounds, AudioFile> mappedSounds_;
};
