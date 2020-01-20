#pragma once

#include "Data/AudioData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/WhistleData.hpp"
#include "Framework/Module.hpp"
#include "Hardware/AudioInterface.hpp"
#include <Tools/Math/FFT.hpp>

#include <boost/circular_buffer.hpp>

class Brain;

/**
 * @class WhistleDetection can detect whether a whistle was whistled while listening.
 * This module will check the microphones during GameState::SET and will detect if whistle
 * was whistled.
 */
class WhistleDetection : public Module<WhistleDetection, Brain>
{
public:
  /// the name of this module
  ModuleName name = "WhistleDetection";
  /**
   * @brief WhistleDetection initializes members
   * @param manager a reference to brain
   */
  WhistleDetection(const ModuleManagerInterface& manager);
  /**
   * @brief cycle detects a whistle in the last recorded audio samples
   */
  void cycle();

private:
  /// audio samples from four microphones that were collected in the last cycle
  const Dependency<RecordData<AudioInterface::numChannels>> recordData_;
  /// the game controller state before whistle integration to run only in SET
  const Dependency<RawGameControllerState> rawGameControllerState_;
  /// the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// information about the whistle status in the current cycle
  Production<WhistleData> whistleData_;

  /// The minimum frequency [Hz] of the whistle band
  const Parameter<double> minFrequency_;
  /// The maximum frequency [Hz] of the whistle band
  const Parameter<double> maxFrequency_;

  /// scale background threshold
  const Parameter<float> backgroundScaling_;
  /// scale whistle threshold
  const Parameter<float> whistleScaling_;

  /// the spectrum is divided into a number of bands to find the whistle band
  const Parameter<unsigned int> numberOfBands_;

  /// the minimum percentage of found  whistles in the whistle buffer required to actually be
  /// considered a detected whistle
  const Parameter<float> minWhistleCount_;

  /// The selected microphone to use for detecting the whistle.
  const Parameter<unsigned int> channel_;

  /// The fft buffer size. For performance, this should be a power of two.
  static constexpr unsigned int fftBufferSize_ = 1024;
  /// FFT wich can transform the buffer
  FFT fft_;
  /// The buffer to store recorded samples until it reaches the fft buffer size and a detection can
  /// be made.
  RealVector fftBuffer_;
  /// the last timestamp when the whistle has been detected
  TimePoint lastTimeWhistleHeard_;

  /// the size of the circular found whistles buffer
  static constexpr unsigned int foundWhistlesBufferSize_ = 4;
  /// circular buffer to store history of found whistles
  boost::circular_buffer<bool> foundWhistlesBuffer_;

  /// The main function that checks whether the buffer contains a whistle sound
  bool fftBufferContainsWhistle();
};
