#pragma once

#include <vector>

#include "Framework/DataType.hpp"

#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Line.hpp"

class FieldBorder : public DataType<FieldBorder>
{
public:
  /**
   * @brief isInsideField
   *
   * Checks if a pixel is inside the field
   *
   * @param p a pixel as a vector of int
   * @return true of false depending on wheater the pixel is inside the field
   *
   * @author Florian Bergmann
   */
  bool isInsideField(const Vector2i& p) const
  {
    for (auto it = border_lines.begin(); it != border_lines.end(); it++)
    {
      float border_m = (float)(it->p2.y() - it->p1.y()) / (it->p2.x() - it->p1.x());
      int border_y = border_m * (p.x() - it->p1.x()) + it->p1.y();
      if (p.y() < border_y)
      {
        return false;
      }
    }
    return true;
  }
  /**
   * @brief getBorderPoints
   *
   * Returns the pixels that define the border with the spacing that is given
   * @param pixel_spacing every n-th pixel that should be returned
   * @return a vector of points that are on the field border
   *
   * @author Florian Bergmann
   */
  const VecVector2i getBorderPoints(int pixel_spacing = 1) const
  {
    VecVector2i border_points;
    Vector2i border_point(0,0);
    int border_y, best_y;
    float border_m;

    for (int i = 0; i < image_size.x(); i += pixel_spacing)
    {
      best_y = 0;

      for (auto it = border_lines.begin(); it != border_lines.end(); it++)
      {
        border_m = (float)(it->p2.y() - it->p1.y()) / (it->p2.x() - it->p1.x());
        border_y = border_m * (i - it->p1.x()) + it->p1.y();

        if (border_y < 0)
        {
          border_y = 0;
        }
        else if (border_y >= image_size.y())
        {
          border_y = image_size.y() - 1;
        }

        if (best_y < border_y)
        {
          border_point.y() = border_y;
          best_y = border_y;
        }
      }
      border_point.x() = i;
      border_points.push_back(border_point);
    }
    return border_points;
  }
  /**
   * @brief reset sets the FieldBorder to a defined state
   */
  void reset()
  {
    valid = false;
    border_lines.clear();
  }
  /// hold all found border lines
  std::vector<Line<int>> border_lines;
  /// needed for getBorderPoints
  Vector2i image_size;
  /// whether the field border is valid
  bool valid;

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["borderLines"] << border_lines;
    value["imageSize"] << image_size;
    value["valid"] << valid;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["borderLines"] >> border_lines;
    value["imageSize"] >> image_size;
    value["valid"] >> valid;
  }
};
