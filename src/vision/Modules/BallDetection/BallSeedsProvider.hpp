#pragma once

#include "Data/BallSeeds.hpp"
#include "Data/CameraMatrix.hpp"
#include "Data/FieldBorder.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/ImageData.hpp"
#include "Data/ImageSegments.hpp"
#include "Framework/Module.hpp"

class Brain;

class BallSeedsProvider : public Module<BallSeedsProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "BallSeedsProvider";
  /**
   * @brief BallSeedsProvider initializes members
   * @param manager a reference to brain
   */
  BallSeedsProvider(const ModuleManagerInterface& manager);

  /**
   * @brief cycle tries to find a ball
   */
  void cycle() override;

private:
  const Dependency<CameraMatrix> cameraMatrix_;
  const Dependency<ImageData> imageData_;
  const Dependency<ImageSegments> imageSegments_;
  const Dependency<FieldBorder> fieldBorder_;
  const Dependency<FieldDimensions> fieldDimensions_;

  /// the maximum Y value a dark segment should have
  const Parameter<int> minSeedBrightDiff_;
  /// the minimum difference to dark segments a bright neighbour pixel should have
  const Parameter<int> seedBrightMin_;
  /// the minimum amount of bright pixels that match seedBright condition
  const Parameter<int> seedBrightScore_;
  const Parameter<int> seedDark_;
  /// the minimal/maximal radiusRatio a dark segment should have
  const Parameter<float> seedRadiusRatioMin_;
  const Parameter<float> seedRadiusRatioMax_;

  void findSeeds(std::vector<BallSeeds::Seed>& seeds) const;

  /// the generated ball candidates
  Production<BallSeeds> ballSeeds_;
};
