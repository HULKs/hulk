#pragma once

#include <Data/AudioData.hpp>
#include <Framework/Module.hpp>
#include <Hardware/AudioInterface.hpp>


class Brain;

class AudioReceiver : public Module<AudioReceiver, Brain>
{
public:
  /**
   * @brief AudioReceiver the constructor initializes the AudioReceiver
   * @param manager a ModuleManagerInterface
   */
  AudioReceiver(const ModuleManagerInterface& manager);
  /**
   * @brief ~AudioReceiver stops audio streaming
   */
  ~AudioReceiver();
  /**
   * @brief cycle transports samples from the hardware audio device
   */
  void cycle();

private:
  /// a reference to the wrapper for the (possibly hardware specific) audio interface
  AudioInterface& audioInterface_;
  /// the recorded samples will be stored in this Production
  Production<RecordData> recordData_;
};
