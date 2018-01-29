#pragma once

#include "Data/SonarData.hpp"
#include "Framework/Module.hpp"


class Motion;

class SonarFilter : public Module<SonarFilter, Motion>
{
public:
  SonarFilter(const ModuleManagerInterface& manager);
  void cycle();

private:
  /// raw sonar sensor data
  const Dependency<SonarSensorData> sonarSensorData_;
  /// filtered sonar sensor data
  Production<SonarData> sonarData_;

  /**
   *  confidentDistance_ sets max distance of sonar readings. All readings above it
   *  will be cut down to confidentDistance_.
   */
  const Parameter<float> confidentDistance_;

  // old data, from the previous cycle
  float oldSonarRight_;
  float oldSonarLeft_;
  float prevRawSonarDataRight_;
  float prevRawSonarDataLeft_;

  // methods
  /// capsulation of filter, to avoid code doubling
  void filter(float& input, float& prevValue);
};
