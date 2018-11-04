#include <cmath>

#include "SonarFilter.hpp"

SonarFilter::SonarFilter(const ModuleManagerInterface& manager)
  : Module(manager)
  , sonarSensorData_(*this)
  , sonarData_(*this)
  , confidentDistance_(*this, "confidentDistance", [] {})
  , invalidReadingsLimit_(*this, "invalidReadingsLimit", [] {})
  , smoothingFactor_(*this, "smoothingFactor", [] {})
  , oldSensorData_{{confidentDistance_(), confidentDistance_()}}
  , invalidDataCounter_{{0, 0}}
{
}

void SonarFilter::cycle()
{
  // Use only the first echo from the left and right sonar sensors,
  // since only the nearest obstacles are relevant for sonar detection.
  const auto& sensorLeft = keys::sensor::SONAR_LEFT_SENSOR_0;
  const auto& sensorRight = keys::sensor::SONAR_RIGHT_SENSOR_0;

  filter(sensorLeft, SONARS::LEFT);
  filter(sensorRight, SONARS::RIGHT);

  debug().update(mount_ + ".invalidDataCounter", invalidDataCounter_);
}

void SonarFilter::filter(keys::sensor::sonar sensorKey, SONARS::SONAR side)
{
  if (sonarSensorData_->valid[sensorKey])
  {
    invalidDataCounter_[side] = 0;
    // Only filter on new sensor readings
    if (oldSensorData_[side] != sonarSensorData_->data[sensorKey])
    {
      lowpass(sonarSensorData_->data[sensorKey], side);
      // save previous raw data for next cycle
      oldSensorData_[side] = sonarSensorData_->data[sensorKey];
    }
  }
  else
  {
    // Count subsequent invalid sensor data
    invalidDataCounter_[side]++;
    if (invalidDataCounter_[side] > invalidReadingsLimit_())
    {
      sonarData_->valid[side] = false;
    }
  }
}

void SonarFilter::lowpass(float measurement, SONARS::SONAR side)
{
  const float lastMeasurement = sonarData_->filteredValues[side];
  // smoothing factor for low-pass using exponential smoothing
  float alpha = smoothingFactor_();
  // Changes in the measured distance greater than this threshold are detected as outliers
  const float outlierThreshold = 0.5;
  // When coming from previously invalid filter output, reinitialize
  // the filter output by completely using the current measurement.
  if (!sonarData_->valid[side])
  {
    sonarData_->valid[side] = true;
    invalidDataCounter_[side] = 0;
    alpha = 1.f;
  }
  else if (std::abs(measurement - lastMeasurement) > outlierThreshold) // Simple outliers detection
  {
    // Apply stronger low-pass filtering to very large changes (outliers)
    // This may sometimes introduce unnecessary delay when the measured distance actually
    // changed that much and not because of noise. Proper outlier detection might want to
    // look at multiple previous values to determine if a large change wasn't actually an outlier.
    alpha = .02f; // This factor can be changed, .02 seems reasonable
  }
  // apply low-pass exponential smoothing
  float filteredOutput = (alpha * measurement) + ((1 - alpha) * lastMeasurement);
  // Clip maximum filter output to the confidentDistance_();
  if (filteredOutput >= confidentDistance_())
  {
    filteredOutput = confidentDistance_();
  }
  sonarData_->filteredValues[side] = filteredOutput;
}
