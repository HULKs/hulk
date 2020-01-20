#pragma once

#include "Framework/Module.hpp"

#include "Data/BallData.hpp"
#include "Data/CameraMatrix.hpp"
#include "Data/FieldColor.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/FilteredSegments.hpp"
#include "Data/ImageData.hpp"
#include "Data/PenaltySpotData.hpp"

class Brain;

class PenaltySpotDetection : public Module<PenaltySpotDetection, Brain>
{
public:
  /// the name of this module
  ModuleName name = "PenaltySpotDetection";
  /**
   * @brief PenaltySpotDetection initializes members
   * @param manager a reference to the brain object
   */
  PenaltySpotDetection(const ModuleManagerInterface& manager);
  /**
   * @brief cycle detects lines and maybe some day circles from the image
   */
  void cycle();

private:
  /**
   * @brief detectPenaltySpot detects the penalty spot on the bottom camera
   */
  void detectPenaltySpot();
  /**
   * @brief sendImagesForDebug send debug information
   */
  void sendImagesForDebug();
  /// Max distance to look for a penalty spot
  const Parameter<float> maxPenaltySpotDetectionDistance_;
  /// Minimum penalty spot pixel radius
  const Parameter<int> minimumPenaltySpotRadius_;
  /// Whether use the chroma check
  const Parameter<bool> requireChromaDiff_;
  /// Whether check if seed is on ball
  const Parameter<bool> excludeBall_;
  /// Distance in x which vertical scanlines are interesting
  const Parameter<int> vScanlineGapToConsider_;
  /// Minimum required difference in luminance
  const Parameter<int> minSpotSeedDiffY_;
  /// Minimum required difference in chroma
  const Parameter<int> minSpotSeedDiffChroma_;
  /// Minimum required intense changes in luminance
  const Parameter<int> significantYSpotSeedPointDiff_;
  /// Minimum required intense changes in chroma
  const Parameter<int> significantChromaSpotSeedPointDiff_;
  /// Minimum number of points with intense diff in luminance
  const Parameter<int> necessarySignificantYSpotSeedPoints_;
  /// Minimum number of points with intense diff in chroma
  const Parameter<int> necessarySignificantChromaSpotSeedPoints_;
  /// Minimum number of points with intense diff in chroma
  const Parameter<bool> requireFieldColor_;
  /// a reference to the image
  const Dependency<ImageData> imageData_;
  /// a reference to the field dimensions
  const Dependency<FieldDimensions> fieldDimensions_;
  /// a reference to the camera matrix
  const Dependency<CameraMatrix> cameraMatrix_;
  /// a reference to the filtered segments
  const Dependency<FilteredSegments> filteredSegments_;
  /// a reference to the ball data
  const Dependency<BallData> ballData_;
  /// a reference to the image segments
  const Dependency<FieldColor> fieldColor_;
  // the detected penalty spot for other mpdules
  Production<PenaltySpotData> penaltySpotData_;
  /// all of the detected penalty spots without clustering
  std::vector<PenaltySpot> penaltySpotSeeds_;
  Vector2i maxPenaltySpotDetectionImagePosition_;
};
