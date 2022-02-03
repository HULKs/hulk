#pragma once

#include "Framework/Module.hpp"

#include "Data/CameraMatrix.hpp"
#include "Data/FieldColor.hpp"
#include "Data/ImageData.hpp"
#include "Data/ImageSegments.hpp"
#include "Data/RobotProjection.hpp"
#include "Vision/Projection/ProjectionCamera.hpp"
#include <set>


class Brain;

class ImageSegmenter : public Module<ImageSegmenter, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"ImageSegmenter"};
  /**
   * ImageSegmenter constructor
   * @param manager a reference to the brain object
   */
  ImageSegmenter(const ModuleManagerInterface& manager);
  void cycle() override;

private:
  struct ScanlineState
  {
    /// the absolute maximum diff
    int maxDiff{0};
    /// the x or y coordinate where the edge intensity was highest
    int peakPosition{0};
    /// the number of sampled points within the last segment
    int scanPoints{0};
    /// the previous y value on the scanline
    std::uint8_t prevYValue{0};
    /// the diff of the previously scanned position
    int prevDiff{0};
    /// the scanline this state belongs to
    Scanline* scanline{nullptr};
  };

  /**
   * @brief initializes the vertical scanlines for a certain image size and number of scanlines
   */
  void initVerticalScanlines();
  /**
   * @brief initializes the horizontal scanlines equidistant in robot coordinates
   */
  void initHorizontalScanlinePositions();

  /**
   * @brief createVerticalScanlines scans the image on vertical scanlines and creates segments which
   * are separated by edges in Y
   * @tparam useMedian if true the median of the pixel's y value and the y values of the pixel
   * above and below is evaluated for segmentation instead of simply the pixel's y value
   */
  template <bool useMedian>
  void createVerticalScanlines();

  /**
   * @brief createHorizontalScanlines scans the image on horizontal scanlines and creates segments
   * which are separated by edges in Y
   */
  void createHorizontalScanlines();

  /**
   * @brief addSegment is a handler for edges that manages segment creation
   * @param peakPosition the coordinate at which the edge has been found
   * @param scanline the scanline on which the edge has been found
   * @param type the type of the detected edge
   * @param scanPoints the number of sampled points within this segment
   */
  void addSegment(const Vector2i& peakPosition, Scanline& scanline, EdgeType type, int scanPoints);

  /**
   * @brief sends debug information of this module
   */
  void sendDebug();

  /// if true the scanline configuration will be updated. Prevents race condition of in-cycle change
  /// of the number of scanlines
  bool updateVerticalScanlines_{true};

  /// this searches for the highest edge intensity in a single monotonic gradient
  void detectEdge(ScanlineState& state, int position, int diff, int edgeThreshold);

  std::array<bool, 2> updateHorizontalScanlines_{true, true};

  const Parameter<bool> drawEdges_;
  const Parameter<bool> drawFieldYellow_;
  const Parameter<bool> drawFullImage_;
  const Parameter<std::array<int, 2>> edgeThresholdHorizontal_;
  const Parameter<std::array<int, 2>> edgeThresholdVertical_;
  const Parameter<int> numVerticalScanlines_;
  const Parameter<float> samplePointDistance_;
  const Parameter<bool> useMedianVerticalTop_;
  const Parameter<bool> useMedianVerticalBottom_;

  const Dependency<CameraMatrix> cameraMatrix_;
  const Dependency<FieldColor> fieldColor_;
  const Dependency<ImageData> imageData_;
  const Dependency<RobotProjection> robotProjection_;

  Production<ImageSegments> imageSegments_;
};
