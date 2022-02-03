#pragma once

#include "Data/SonarData.hpp"
#include "Framework/Module.hpp"


class Motion;

class SonarFilter : public Module<SonarFilter, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"SonarFilter"};
  explicit SonarFilter(const ModuleManagerInterface& manager);
  void cycle() override;

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
  SonarsArray<float> oldSensorData_;
  /// Counts for subsequent invalid sensor readings
  SonarsArray<unsigned int> invalidDataCounter_;
  SonarsArray<std::list<float>> medianWindow_;

  /**
   * @brief Check sensor data validity and apply a filter according to configuration.
   * @param sonar The sensor to use for filtering
   * @param measuremnt the new data read from the sensor
   */
  void filter(Sonars sonar, float measurement);
  /**
   * @brief Low-pass filter for the raw sonar data using exponential smoothing
   * @param sonar left or right sonar side to use
   * @param measurement Raw sensor measurement
   */
  void lowpass(Sonars sonar, float measurement);
  /**
   * @brief Median filter for the raw sonar data using a median filter.
   * @param sonar left or right sonar side to use
   * @param measurement Raw sensor measurement
   */
  void median(Sonars sonar, float measurement);
};
