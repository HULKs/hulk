#pragma once

#include <vector>

#include "Data/CameraMatrix.hpp"
#include "Data/FieldBorder.hpp"
#include "Data/FilteredSegments.hpp"
#include "Data/ImageData.hpp"
#include "Data/ImageSegments.hpp"

#include "Framework/Module.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Storage/Image.hpp"

class Brain;

/**
 * @brief The FieldBorderDetection class
 *
 * This class takes all found field segments and marks the top points as potential field border
 * points. Using the RANSAC algorithm, the best line for all field border points is determined. If
 * there are enought points left, a second and a third line will be detected.
 *
 * @author Florian Bergmann
 */
class FieldBorderDetection : public Module<FieldBorderDetection, Brain>
{
public:
  /// the name of this module
  ModuleName name = "FieldBorderDetection";
  /**
   * @brief FieldBorderDetection constructor
   *
   * @param manager a reference to the brain object
   *
   * @author Florian Bergmann
   */
  FieldBorderDetection(const ModuleManagerInterface& manager);
  /**
   * @brief cycle
   *
   * Determines the border points and uses RANSAC to find the field border lines
   *
   * @author Florian Bergmann
   */
  void cycle();

private:
  /**
   * @brief findBorderPoints
   *
   * Takes field segments and saves the start point with the lowest y coordinate as border points
   * Due to the coordinate system of the image (see below) the point with the lowest y-coordinate
   * is the highest point in the image. The sorting of the field segments allow to use the previous
   * point when the x-coordinate of the current point is different.
   *
   * 0------------> X
   * |
   * |
   * |
   * |
   * |
   * v
   * Y
   *
   * @author Florian Bergmann
   */
  void findBorderPoints();
  /**
   * @brief isOrthogonal
   *
   * Checks if two lines are orthogonal
   *
   * @param l1 first line
   * @param l2 second line
   * @return true or false depending on wheater the two lines are orthogonal or not
   *
   * @author Florian Bergmann
   */
  bool isOrthogonal(const Line<int>& l1, const Line<int>& l2);
  /**
   * @brief centerOfGroup
   *
   * Finds the point in the center of a group of points
   *
   * @param group
   * @return the point that is in the center
   *
   * @author Florian Bergmann
   */
  Vector2i centerOfGroup(VecVector2i group);
  /**
   * @brief bestFitLine
   *
   * Calculates the best fit line for a group of points
   *
   * @param points
   * @return the Line that fits best
   *
   * @author Florian Bergmann
   */
  Line<int> bestFitLine(VecVector2i points);
  /**
   * @brief findBorderLines
   *
   * Searches for the possible field lines using the RANSAC algorithm
   *
   * @author Florian Bergmann
   */
  void findBorderLines();
  /**
   * ransac for lines
   */
  bool ransac(Line<int>& bestLine, const VecVector2<int>& points, VecVector2<int>& best,
              VecVector2<int>& unused, unsigned int iterations, int max_distance);
  /**
   * @brief createFilteredSegments creates a version of the segments that contains only
   * the segments below the field border and that are not part of the field
   */
  void createFilteredSegments();
  /**
   * @brief sendImagesForDebug
   *
   * Used for debugging only
   *
   * @author Florian Bergmann
   */
  void sendImagesForDebug();
  /// holds all found border points
  VecVector2i borderPoints_;
  /// deviaten threshold for the 90 degree corners of the field borders
  const Parameter<int> angleThreshold_;
  /// the minimum amount of points a line has to contain to be considered as field border
  const Parameter<int> minPointsPerLine_;
  const Parameter<bool> drawVerticalFilteredSegments_;
  const Parameter<bool> drawHorizontalFilteredSegments_;
  const Parameter<bool> drawVerticalEdges_;
  const Parameter<bool> drawHorizontalEdges_;
  /// a reference to the currently processed image
  const Dependency<ImageData> imageData_;
  /// a reference to the result of the image segmentation
  const Dependency<ImageSegments> imageSegments_;
  /// a reference to the camera matrix
  const Dependency<CameraMatrix> cameraMatrix_;
  /// the result of the field border detection
  Production<FieldBorder> fieldBorder_;
  /// the segments that are below the field border and no field
  Production<FilteredSegments> filteredSegments_;
};
