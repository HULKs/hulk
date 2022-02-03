#pragma once

#include <cmath>

#define TO_RAD static_cast<float>(M_PI / 180.f)


namespace Angle
{
  /**
   * @brief angleDiff calculates the absolute diff of angles
   * @param a 1st angle
   * @param b 2nd angle
   * @return absolute difference between angles, is between zero and PI, shortest way
   */
  inline float angleDiff(const float a, const float b)
  {
    float phi = std::fmod(std::abs(a - b), 2 * M_PI);
    return phi > M_PI ? 2 * M_PI - phi : phi;
  }
  /**
   * @brief normalizeAngleDiff normalizes a given angle difference
   * @param angleDiff the angle diff to be normalized
   * @return the normalized angle diff
   */
  inline float normalizeAngleDiff(const float angleDiff)
  {
    float phi = std::fmod(angleDiff, 2 * M_PI);
    if (phi > M_PI)
    {
      return phi - 2 * M_PI;
    }
    else if (phi < -M_PI)
    {
      return 2 * M_PI + phi;
    }
    else
    {
      return phi;
    }
  }
  /**
   * @brief normalzed normalizes an angle to the range ]-pi , pi]
   * @param angle the angle to be normalized
   * @return the normalized angle
   */
  inline float normalized(const float angle)
  {
    if (angle == static_cast<float>(M_PI))
    {
      return M_PI;
    }
    return angle - 2 * M_PI * std::floor(angle / (2 * M_PI) + 0.5f);
  }
} // namespace Angle
