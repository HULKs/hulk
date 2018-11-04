#pragma once

#include <vector>

#include "Framework/DataType.hpp"

#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Line.hpp"

class FieldBorder : public DataType<FieldBorder>
{
public:
  /// the name of this DataType
  DataTypeName name = "FieldBorder";
  /**
   * @brief isInsideField
   *
   * Checks if a pixel is inside the field
   *
   * @param p a pixel as a vector of int
   * @return whether the pixel is inside the field
   *
   * @author Florian Bergmann
   */
  bool isInsideField(const Vector2i& p) const
  {
    for (auto it = borderLines.begin(); it != borderLines.end(); it++)
    {
      float borderM = (float)(it->p2.y() - it->p1.y()) / (it->p2.x() - it->p1.x());
      int borderY = borderM * (p.x() - it->p1.x()) + it->p1.y();
      if (p.y() < borderY)
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
   * @param pixelSpacing every n-th pixel that should be returned
   * @return a vector of points that are on the field border
   *
   * @author Florian Bergmann
   */
  const VecVector2i getBorderPoints(int pixelSpacing = 1) const
  {
    VecVector2i borderPoints;
    Vector2i borderPoint(0, 0);
    int borderY, bestY;
    float borderM;

    for (int i = 0; i < imageSize.x(); i += pixelSpacing)
    {
      bestY = 0;

      for (auto it = borderLines.begin(); it != borderLines.end(); it++)
      {
        borderM = (float)(it->p2.y() - it->p1.y()) / (it->p2.x() - it->p1.x());
        borderY = borderM * (i - it->p1.x()) + it->p1.y();

        if (borderY < 0)
        {
          borderY = 0;
        }
        else if (borderY >= imageSize.y())
        {
          borderY = imageSize.y() - 1;
        }

        if (bestY < borderY)
        {
          borderPoint.y() = borderY;
          bestY = borderY;
        }
      }
      borderPoint.x() = i;
      borderPoints.push_back(borderPoint);
    }
    return borderPoints;
  }
  /**
   * @brief reset sets the FieldBorder to a defined state
   */
  void reset()
  {
    valid = false;
    borderLines.clear();
  }
  /// hold all found border lines
  std::vector<Line<int>> borderLines;
  /// needed for getBorderPoints
  Vector2i imageSize = Vector2i::Zero();
  /// whether the field border is valid
  bool valid = false;

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["borderLines"] << borderLines;
    value["imageSize"] << imageSize;
    value["valid"] << valid;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["borderLines"] >> borderLines;
    value["imageSize"] >> imageSize;
    value["valid"] >> valid;
  }
};
