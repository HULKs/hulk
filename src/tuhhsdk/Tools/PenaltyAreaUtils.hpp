#pragma once

#include "Data/FieldDimensions.hpp"
#include "Framework/Module.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Hysteresis.hpp"


class PenaltyAreaUtils
{
public:
  /**
   * Checks if a given position is inside a penalty area
   * @param position Coordinates as Vector2f
   * @param fieldDimensions The fieldDimensions
   * @param hysteresis The hysteresis factor
   * @param savedState The saved value
   * @return The new value
   */
  static bool isInPenaltyArea(const Vector2f& position,
                              const Dependency<FieldDimensions>& fieldDimensions,
                              const float& hysteresis, const bool& savedState)
  {
    return Hysteresis<float>::smallerThan(std::abs(position.x()),
                                          fieldDimensions->fieldLength / 2 + hysteresis, hysteresis,
                                          savedState) &&
           Hysteresis<float>::greaterThan(std::abs(position.x()),
                                          fieldDimensions->fieldLength / 2 -
                                              fieldDimensions->fieldPenaltyAreaLength - hysteresis,
                                          hysteresis, savedState) &&
           Hysteresis<float>::smallerThan(std::abs(position.y()),
                                          fieldDimensions->fieldPenaltyAreaWidth / 2 + hysteresis,
                                          hysteresis, savedState);
  }
};
