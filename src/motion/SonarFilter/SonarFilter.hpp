#pragma once

#include <Modules/NaoProvider.h>
#include "Data/SonarData.hpp"
#include "Framework/Module.hpp"


class Motion;

class SonarFilter : public Module<SonarFilter, Motion>
{
public:
  /// the name of this module
  ModuleName name = "SonarFilter";
  SonarFilter(const ModuleManagerInterface& manager);
  void cycle();

private:
  /// raw sonar sensor data
  const Dependency<SonarSensorData> sonarSensorData_;
  /// filtered sonar sensor data
  Production<SonarData> sonarData_;

  /**
   *  confidentDistance_ sets max distance of sonar readings.
   *  All readings above it will be clipped to confidentDistance_.
   */
  const Parameter<float> confidentDistance_;
  /// Too many subsqeuent invalid sensor readings will invalidate the filter output.
  const Parameter<unsigned int> invalidReadingsLimit_;
  /// smoothing factor for low-pass using exponential smoothing. Lower values mean more smoothing.
  const Parameter<float> smoothingFactor_;
  const Parameter<float> medianWindowSize_;
  const Parameter<bool> useMedian_;

  /// last raw data from the previous cycle
  std::array<float, 2> oldSensorData_;
  /// Counts for subsequent invalid sensor readings
  std::array<unsigned int, 2> invalidDataCounter_;
  std::array<std::list<float>, 2> medianWindow_;

  /**
   * @brief Check sensor data validity and apply a filter according to configuration.
   * @param sensorKey The sensor echo to use for filtering
   * @param side specifies which side the sensor echo belongs to, left or right
   */
  void filter(keys::sensor::sonar sensorKey, SONARS::SONAR side);
  /**
   * @brief Low-pass filter for the raw sonar data using exponential smoothing
   * @param measurement Raw sensor measurement
   * @param side left or right sonar side to use
   */
  void lowpass(float measurement, SONARS::SONAR side);
  /**
   * @brief Median filter for the raw sonar data using a median filter.
   * @param measurement Raw sensor measurement
   * @param side left or right sonar side to use
   */
  void median(float measurement, SONARS::SONAR side);
};
