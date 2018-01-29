#include <cmath>

#include "SonarFilter.hpp"


SonarFilter::SonarFilter(const ModuleManagerInterface& manager)
  : Module(manager, "SonarFilter")
  , sonarSensorData_(*this)
  , sonarData_(*this)
  , confidentDistance_(*this, "confidentDistance", [] {})
  , oldSonarRight_(1)
  , oldSonarLeft_(1)
  , prevRawSonarDataRight_(0)
  , prevRawSonarDataLeft_(0)
{
}

void SonarFilter::cycle()
{
  // get latest raw data
  sonarData_->sonarLeft = sonarSensorData_->sonarLeft;
  sonarData_->sonarRight = sonarSensorData_->sonarRight;

  // if sensor has new data
  if (prevRawSonarDataLeft_ != sonarData_->sonarLeft || prevRawSonarDataRight_ != sonarData_->sonarRight)
  {
    // save previous raw data
    prevRawSonarDataLeft_ = sonarData_->sonarLeft;
    prevRawSonarDataRight_ = sonarData_->sonarRight;

    // left
    filter(sonarData_->sonarLeft, oldSonarLeft_);
    // right
    filter(sonarData_->sonarRight, oldSonarRight_);

    // save filtered data for next cycle
    oldSonarLeft_ = sonarData_->sonarLeft;
    oldSonarRight_ = sonarData_->sonarRight;

    // send debug data, only if filtering takes place.
    debug().update(mount_ + ".SonarData", *sonarData_);
  }
}

void SonarFilter::filter(float& input, float& prevValue)
{
  float factor = 0.25f; // default lowpass factor

  // ignore not valid measurement values according to the nao v5 specs
  if (input <= 0)
  {
    input = prevValue;
  }

  // fast changes (>0.5/cycle) have even lower impact.
  if (std::abs(input - prevValue) > 0.5f)
  {
    factor = .02f; // This factor can be changed, .02 seems reasonable
  }

  // apply low pass
  input = (factor * input) + ((1 - factor) * prevValue);


  // set all values larger than confidentDistance (config parameter) to be confidentDistance_();
  if (input >= confidentDistance_())
  {
    input = confidentDistance_();
  }
}
