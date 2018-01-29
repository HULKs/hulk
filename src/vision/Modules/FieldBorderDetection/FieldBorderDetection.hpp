#pragma once

#include <vector>

#include "Data/CameraMatrix.hpp"
#include "Data/FieldBorder.hpp"
#include "Data/FilteredRegions.hpp"
#include "Data/ImageData.hpp"
#include "Data/ImageRegions.hpp"

#include "Framework/Module.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Storage/Image.hpp"

class Brain;

/**
 * @brief The FieldBorderDetection class
 *
 * This class takes all found field regions and makrs the top points as potential field border points.
 * Using the RANSAC algorithm, the best line for all field border points is determined.
 * If there are enought points left, a second and a third line will be detected.
 *
 * @author Florian Bergmann
 */
class FieldBorderDetection : public Module<FieldBorderDetection, Brain>
{
public:
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
   * Takes field regions and saves the start point with the lowest y coordinate as border points
   * Due to the coordinate system of the image (see below) the point with the lowest y-coordinate
   * is the highest point in the image. The sorting of the field regions allow to use the previous
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
   * @brief createFilteredRegions creates a version of the regions that contains only
   * the regions below the field border and that are not part of the field
   */
  void createFilteredRegions();
  /**
   * @brief sendImagesForDebug
   *
   * Used for debugging only
   *
   * @author Florian Bergmann
   */
  void sendImagesForDebug();
  /// holds all found border points
  VecVector2i border_points_;
  /// deviaten threshold for the 90 degree corners of the field borders
  const Parameter<int> angle_threshold_;
  /// a reference to the currently processed image
  const Dependency<ImageData> image_data_;
  /// a reference to the result of the image segmentation
  const Dependency<ImageRegions> image_regions_;
  /// a reference to the camera matrix
  const Dependency<CameraMatrix> camera_matrix_;
  /// the result of the field border detection
  Production<FieldBorder> field_border_;
  /// the regions that are below the field border and no field
  Production<FilteredRegions> filtered_regions_;
};
