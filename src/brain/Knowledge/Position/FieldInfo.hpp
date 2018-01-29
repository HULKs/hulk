#pragma once

#include <vector>

#include "Data/FieldDimensions.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Line.hpp"

class FieldInfo
{
public:
  /**
   * @brief FieldInfo constructs an array of lines and goalPosts from the dimensions stored in the FieldDimensions
   * @param playerConfiguration a reference to the player configuration
   * @param fieldDimensions a reference to the field dimensions
   */
  FieldInfo(const PlayerConfiguration& playerConfiguration, const FieldDimensions& fieldDimensions);
  /// contains all the lines on the field
  std::vector<Line<float>> lines;
  /// contains all the goal posts on the field
  VecVector2f goalPosts;

private:
  /**
   * @brief polar2cartesian converts a vector in polar coordinates to cartesian coordinates
   * @param polar a vector containing the radius and the angle (r, phi)
   * @return a vector in cartesian coordinates (x, y)
   */
  Vector2f polar2cartesian(const Vector2f& polar) const;
  /// a reference to the player configuration
  const PlayerConfiguration& playerConfiguration_;
};
