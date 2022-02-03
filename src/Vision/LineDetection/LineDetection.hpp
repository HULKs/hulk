#pragma once

#include "Framework/Module.hpp"
#include "Tools/Math/Line.hpp"

#include "Data/CameraMatrix.hpp"
#include "Data/FilteredSegments.hpp"
#include "Data/ImageData.hpp"
#include "Data/LineData.hpp"

class Brain;

class LineDetection : public Module<LineDetection, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"LineDetection"};
  /**
   * @brief LineDetection initializes members
   * @param manager a reference to the brain object
   */
  explicit LineDetection(const ModuleManagerInterface& manager);
  /**
   * @brief cycle detects lines and maybe some day circles from the image
   */
  void cycle() override;

private:
  /**
   * @brief getGradient calculates the normalized gradient in the y channel
   * @param p a point in pixel coordinates at which the gradient is computed
   * @return the gradient
   */
  Vector2f getGradient(const Vector2i& p) const;
  /**
   * @brief detectLinePoints uses the scanline segments and detects points which could belong to a
   * line
   */
  void detectLinePoints();
  /**
   * @brief hasReasonableSize checks whether the projected segment size is reasonable for a line
   * @param segment the segment to check
   * @return whether is has reasonable size
   */
  bool hasReasonableSize(const Segment& segment) const;
  /**
   * @brief checkLength checks the length of line (the number of points as well as the distance
   * between start and end)
   * @param line a vector of points that form a line
   * @return true iff the line is sufficiently long
   */
  bool checkLength(const VecVector2i& linePoints) const;
  /**
   * @brief correctEndpoints ensures best line endpoints and their order
   */
  static void correctEndpoints(Line<int>& line, const VecVector2i& linePoints);
  /**
   * @brief correctLine calculates better line endpoints, checks the line for holes, splits it up if
   * necessary and adds it to the list of lines
   * @param line a vector of points that form a line
   * @param unused a vector of points that are not used for a line
   * @return true iff the points really form a line
   */
  bool correctLine(Line<int> detectedLine, VecVector2i& linePoints, VecVector2i& unusedPoints);
  /**
   * @brief ransacHandler handles the ransac output and the remaining points on which lines can
   * still be detected
   */
  void ransacHandler();
  /**
   * ransac for lines
   */
  static bool ransac(Line<int>& bestLine, const VecVector2<int>& points, VecVector2<int>& best,
                     VecVector2<int>& unused, unsigned int iterations, int maxDistance);
  /**
   * @brief createLineData converts the internallty found lines to the exposed LineData class
   */
  void createLineData();
  /**
   * @brief sendImagesForDebug send debug information
   */
  void sendImagesForDebug();
  /// the maximum allowed gap (in pixels) within a line
  const Parameter<int> maxGapOnLine_;
  /// the maximum allowed distance (in pixels) of a point from a line
  const Parameter<int> maxDistFromLine_;
  /// the minimum number of points per line
  const Parameter<unsigned int> minNumberOfPointsOnLine_;
  /// the minimum allowed length of a line
  const Parameter<int> minPixelLength_;
  /// whether the projected segment size should be checked
  const Parameter<bool> checkLineSegmentsProjection_;
  /// max projected line segment size
  const Parameter<float> maxProjectedLineSegmentLength_;
  /// a reference to the image
  const Dependency<ImageData> imageData_;
  /// a reference to the camera matrix
  const Dependency<CameraMatrix> cameraMatrix_;
  /// a reference to the filtered segments
  const Dependency<FilteredSegments> filteredSegments_;
  /// the detected lines for other modules
  Production<LineData> lineData_;
  // line points for debug puroposes
  VecVector2i debugLinePoints_;
  /// candidate points on lines
  VecVector2i linePoints_;
  /// detected lines
  std::vector<Line<int>> lines_;
};
