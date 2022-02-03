#include "Motion/SonarFilter/SonarFilter.hpp"
#include <cmath>

SonarFilter::SonarFilter(const ModuleManagerInterface& manager)
  : Module(manager)
  , sonarSensorData_(*this)
  , sonarData_(*this)
  , confidentDistance_(*this, "confidentDistance", [] {})
  , invalidReadingsLimit_(*this, "invalidReadingsLimit", [] {})
  , smoothingFactor_(*this, "smoothingFactor", [] {})
  , medianWindowSize_(*this, "medianWindowSize", [] {})
  , useMedian_(*this, "useMedian", [] {})
  , oldSensorData_{{confidentDistance_(), confidentDistance_()}}
  , invalidDataCounter_{{0, 0}}
{
}

void SonarFilter::cycle()
{
  // Use only the first echo from the left and right sonar sensors,
  // since only the nearest obstacles are relevant for sonar detection.
  filter(Sonars::LEFT, sonarSensorData_->data.leftSensor);
  filter(Sonars::RIGHT, sonarSensorData_->data.rightSensor);

  debug().update(mount_ + ".invalidDataCounter", invalidDataCounter_);
}

void SonarFilter::filter(Sonars sonar, float measurement)
{
  if (sonarSensorData_->valid[sonar])
  {
    invalidDataCounter_[sonar] = 0;
    // Only filter on new sensor readings
    if (oldSensorData_[sonar] != measurement)
    {
      if (useMedian_())
      {
        median(sonar, measurement);
      }
      else
      {
        lowpass(sonar, measurement);
      }
      // save previous raw data for next cycle
      oldSensorData_[sonar] = measurement;
    }
  }
  else
  {
    // Count subsequent invalid sensor data
    invalidDataCounter_[sonar]++;
    if (invalidDataCounter_[sonar] > invalidReadingsLimit_())
    {
      sonarData_->valid[sonar] = false;
      // Set the filtered value anyway for a less confusing debug graph
      sonarData_->filteredValues[sonar] = confidentDistance_();
    }
  }
}

void SonarFilter::lowpass(Sonars sonar, float measurement)
{
  const float lastMeasurement = sonarData_->filteredValues[sonar];
  // smoothing factor for low-pass using exponential smoothing
  float alpha = smoothingFactor_();
  // Changes in the measured distance greater than this threshold are detected as outliers
  const float outlierThreshold = 0.5;
  // When coming from previously invalid filter output, reinitialize
  // the filter output by completely using the current measurement.
  if (!sonarData_->valid[sonar])
  {
    sonarData_->valid[sonar] = true;
    invalidDataCounter_[sonar] = 0;
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
  sonarData_->filteredValues[sonar] = filteredOutput;
}

void SonarFilter::median(Sonars sonar, float measurement)
{
  if (!sonarData_->valid[sonar])
  {
    sonarData_->valid[sonar] = true;
    invalidDataCounter_[sonar] = 0;
  }
  // Keep data window at a maximum size
  while (medianWindow_[sonar].size() >= medianWindowSize_())
  {
    medianWindow_[sonar].pop_back();
  }
  medianWindow_[sonar].insert(medianWindow_[sonar].begin(), measurement);
  // Get median value
  std::list<float> tmp = medianWindow_[sonar];
  tmp.sort();
  auto median = tmp.begin();
  std::advance(median, static_cast<size_t>(tmp.size() / 2));
  // Ignore values above confidentDistance threshold
  if (*median < confidentDistance_())
  {
    sonarData_->filteredValues[sonar] = *median;
  }
  else
  {
    sonarData_->filteredValues[sonar] = confidentDistance_();
  }
}
