#pragma once

#include "Framework/Module.hpp"
#include "Tools/Math/Line.hpp"

#include "Data/CameraMatrix.hpp"
#include "Data/CircleData.hpp"
#include "Data/FilteredRegions.hpp"
#include "Data/ImageData.hpp"
#include "Data/LineData.hpp"

class Brain;

class FieldMarksDetection : public Module<FieldMarksDetection, Brain>
{
public:
  /**
   * @brief FieldMarksDetection initializes members
   * @param manager a reference to the brain object
   */
  FieldMarksDetection(const ModuleManagerInterface& manager);
  /**
   * @brief cycle detects lines and maybe some day circles from the image
   */
  void cycle();

private:
  /**
   * @brief getGradient calculates the normalized gradient in the y channel
   * @param p a point in pixel coordinates at which the gradient is computed
   * @return the gradient
   */
  Vector2f getGradient(const Vector2i& p) const;
  /**
   * @brief detectLinePoints uses the scanline regions and detects points which could belong to a line
   */
  void detectLinePoints();
  /**
   * @brief isIlluminated transforms the pixel into an illumination invariant space to consider between an illuminated spot on the field or something else
   * @param x the pixel x coordinate
   * @param y the pixel y coordinate
   * @return boolean if spot is illuminated
   */
  bool isIlluminated(const unsigned int x, const unsigned int y) const;
  /**
   * @brief checkLength checks the length of line (the number of points as well as the distance between start and end)
   * @param line a vector of points that form a line
   * @return true iff the line is sufficiently long
   */
  bool checkLength(const VecVector2i& linePoints) const;
  Vector2i getOrthogonalPixelProjection(const Vector2i& v, Line<int>& line);
  void correctEndpoints(Line<int>& line, const VecVector2i& linePoints);
  /**
   * @brief checkLine checks the line for holes, splits it up if necessary and adds it to the list of lines
   * @param line a vector of points that form a line
   * @param unused a vector of points that are not used for a line
   * @return true iff the points really form a line
   */
  bool correctLine(Line<int> detectedLine, VecVector2i& linePoints, VecVector2i& unusedPoints);
  /**
   * @brief ransac runs RANSAC to fit lines into the line points and uses checkLine to check/split the found lines further
   */
  void ransac();
  /**
   * @brief createLineData converts the internallty found lines to the exposed LineData class
   */
  void createLineData();
  /// the maximum allowed gap (in pixels) within a line
  const Parameter<int> maxGapOnLine_;
  /// the maximum allowed distance (in pixels) of a point from a line
  const Parameter<int> maxDistFromLine_;
  /// the minimum number of points per line
  const Parameter<unsigned int> minNumberOfPointsOnLine_;
  /// the minimum allowed length of a line
  const Parameter<float> minPixelLength_;
  /// whether the daylight filter should be used
  const Parameter<bool> useDaylightFilter_;
  /// lower threshold to classify more illuminated areas as field
  const Parameter<double> daylightThreshold_;
  /// a reference to the image
  const Dependency<ImageData> image_data_;
  /// a reference to the camera matrix
  const Dependency<CameraMatrix> camera_matrix_;
  /// a reference to the filtered regions
  const Dependency<FilteredRegions> filtered_regions_;
  /// the detected lines for other modules
  Production<LineData> line_data_;
  /// the detected circle for other modules
  Production<CircleData> circle_data_;
  /// candidate points on lines
  VecVector2i line_points_;
  /// detected lines
  std::vector<Line<int>> lines_;
};
