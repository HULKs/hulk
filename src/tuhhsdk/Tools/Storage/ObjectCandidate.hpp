#pragma once

#include "Tools/Math/Circle.hpp"
#include "Tools/Storage/Image.hpp"
#include "Tools/Storage/UniValue/UniValue.h"
#include <vector>

/**
 * @brief DebugCandidate is a combination of a candidate and a color in which the candidate should
 * be drawn
 */
template <typename T>
struct DebugCandidate
{
  /**
   * @brief DebugCandidate initializes a debug circle with given circle and color
   * @param circle a circle in image coordinates
   * @param color the color in which to draw the candidate
   */
  DebugCandidate(const T& candidate, const Color& color)
    : candidate(candidate)
    , color(color)
  {
  }

  /// a candidate
  T candidate;
  /// the color in which to draw the candidate
  Color color;
};

struct ObjectCandidate : public Circle<int>
{
  ObjectCandidate(const Circle<int>& circle, const std::vector<std::uint8_t> sample)
    : Circle<int>(circle)
    , sample(std::move(sample))
  {
  }

  ObjectCandidate(const Vector2i& center, const int& radius, const std::vector<std::uint8_t> sample)
    : Circle<int>(center, radius)
    , sample(std::move(sample))
  {
  }

  /// the sample image (pixels) of this candidate
  const std::vector<std::uint8_t> sample;
};
