#pragma once

#include "Data/AudioData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/WhistleData.hpp"
#include "Framework/Module.hpp"
#include <Tools/Math/FFT.hpp>

class Brain;

/**
 * @class WhistleDetection can detect whether a whistle was whistled while listening.
 * This module will check the microphones during GameState::SET and will detect if  whistle
 * was whistled.
 *
 * Currently, a band is defined via configuration that is considered the whistle band. In this band
 * the power spectral density (PSD) is calculated and compared to the PSD in the higher frequencies.
 * If the relationship between whistle-power and non-whistle-power exceeds a configured threshold,
 * the whistle is considered detected.
 */
class WhistleDetection : public Module<WhistleDetection, Brain>
{
public:
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
  /// audio samples that were collected in the last cycle
  const Dependency<RecordData> recordData_;
  /// the game controller state before whistle integration to run only in SET
  const Dependency<RawGameControllerState> rawGameControllerState_;
  /// the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// information about the whistle status in the current cycle
  Production<WhistleData> whistleData_;

  /// The minimum frequency [Hz] of the whistle band
  Parameter<double> minFrequency_;
  /// The maximum frequency [Hz] of the whistle band
  Parameter<double> maxFrequency_;
  /// The threshold of the quotient of whistle-power divided by non-whistle-power for which the
  /// whistle is considered detected.
  Parameter<double> threshold_;

  /// The buffer size. For performance, this should be a power of two.
  static constexpr unsigned int BUFFER_SIZE = 8192;
  /// The sampling rate. Depends on samplingRate of the audio recording.
  static constexpr double samplingRate = 44100;
  /// FFT wich can transform the buffer
  FFT fft_;
  /// The buffer to store recorded samples until it reaches the BUFFER_SIZE and a detection
  /// can be made.
  RealVector buffer_;
  /// the last timestamp when the whistle has been detected
  TimePoint lastTimeWhistleHeard_;

  /// The main function that checks whether the buffer contains a whistle sound
  bool bufferContainsWhistle();
};
