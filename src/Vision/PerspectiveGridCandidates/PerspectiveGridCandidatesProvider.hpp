#pragma once

#include "Data/CameraMatrix.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/FilteredSegments.hpp"
#include "Data/ImageData.hpp"
#include "Data/LineData.hpp"
#include "Data/PerspectiveGridCandidates.hpp"
#include "Framework/Module.hpp"
#include <vector>

class Brain;

/*
 * @brief This module fills the image (from the highest Y-position to the horizon) with a
 * perspective grid of boxes/circles: At each Y-position a radius is calculated based on the
 * projection. Radii are placed line-by-line upwards (positive Y-direction) on the image until the
 * horizon is reached or the radius size becomes too small. In the second step, this module only
 * keeps the boxes/circles where a center point of a vertical filter segment exists. The remaining
 * boxes are passed to the ball detection.
 */
class PerspectiveGridCandidatesProvider : public Module<PerspectiveGridCandidatesProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"PerspectiveGridCandidatesProvider"};
  /**
   * @brief PerspectiveGridCandidatesProvider initializes members
   * @param manager a reference to brain
   */
  explicit PerspectiveGridCandidatesProvider(const ModuleManagerInterface& manager);

  /**
   * @brief cycle generates candidates based on vertical filtered segments
   */
  void cycle() override;

  /**
   * @brief generates perspective rows of circles by iterating over image in y-axis
   */
  void generateCircleRows();

  /**
   * @brief generates candidates by iterating over filtered segments and associating them a
   * candidate circle
   */
  void generateCandidates();

  /**
   * @brief sends the debug image showing candidates
   */
  void sendDebugImage() const;

private:
  /// current image to find the ball
  const Dependency<ImageData> imageData_;
  const Dependency<CameraMatrix> cameraMatrix_;
  /// contains the ballSize
  const Dependency<FieldDimensions> fieldDimensions_;
  /// a reference to the filtered segments
  const Dependency<FilteredSegments> filteredSegments_;
  /// a reference to the detected lines (for using center circles)
  const Dependency<LineData> lineData_;

  /// the minimum radius of generated circles in 444 pixels (this should be set larger than a few
  /// pixels, otherwise the candidate generator generates many or infinite circle rows, e.g. tune it
  /// to be the smallest ball at maximal distance that should be detected)
  const Parameter<int> minimumRadius_;
  /// the maximum amount of generated circles in 444 pixels
  const Parameter<std::size_t> maximumCandidates_;

  // the generated perspective-grid candidates
  Production<PerspectiveGridCandidates> perspectiveGridCandidates_;

  /// stores one row of circles
  struct CircleRow
  {
    /// the y-position of the circle center
    int centerLineY;
    /// the radius at the center line in 444 coordinates
    int radius444;

    CircleRow(int centerLineY, int radius444)
      : centerLineY{centerLineY}
      , radius444{radius444}
    {
    }
  };

  /// the y-coordinate of the horizon
  int horizonY_;
  /// the generated rows of circles
  std::vector<CircleRow> circleRows_;
  /// the upper bound of circles that can be generated at maximum
  int numberOfCircles_;
};
