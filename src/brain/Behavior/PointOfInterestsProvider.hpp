#pragma once

#include "Data/FieldDimensions.hpp"
#include "Data/PointOfInterests.hpp"
#include "Data/RobotPosition.hpp"
#include "Framework/Module.hpp"

class Brain;

/**
 * @brief Module description
 */
class PointOfInterestsProvider : public Module<PointOfInterestsProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "PointOfInterestsProvider";
  /**
   *@brief The constructor of this class
   */
  explicit PointOfInterestsProvider(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<RobotPosition> robotPosition_;

  const Parameter<float> centerCircleWeight_;
  const Parameter<float> penaltyAreaWeight_;
  const Parameter<float> tIntersectionCenterLineWeight_;
  const Parameter<float> penaltyAreaCornerWeight_;
  const Parameter<float> cornerWeight_;
  const Parameter<float> maxPOIDistance_;
  Parameter<float> maxPOIAngle_;

  Production<PointOfInterests> pointOfInterests_;

  void fillAbsolutePOIs();
  void findBestPOI();
};
