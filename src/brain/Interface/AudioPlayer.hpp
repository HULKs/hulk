#pragma once

#include <Data/AudioData.hpp>
#include <Framework/Module.hpp>
#include <Hardware/AudioInterface.hpp>

class Brain;

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
  void cycle();

private:
  /// a reference to the wrapper for the (possibly hardware specific) audio interface
  AudioInterface& audioInterface_;
  /// This data will be played back via the audioInterface_.
  Dependency<PlaybackData> playbackData_;
};
