#pragma once

namespace Hysteresis
{
  /**
   * @brief checks if the first operand is greater than the second including hysteresis.
   * @param first The fist operand
   * @param second The second operand
   * @param hysteresis The hysteresis factor
   * @param wasGreater if the first operand was greater than the second one last time.
   * @return true if the first operand is greater than the second one after hysteresis was applied.
   */
  template <typename Value, typename Hysteresis>
  constexpr bool greaterThan(const Value& first, const Value& second, const Hysteresis& hysteresis,
                             const bool wasGreater)
  {
    if (wasGreater)
    {
      return first > second - hysteresis;
    }
    return first > second + hysteresis;
  }
  /**
   * @brief checks if the first operand is smaller than the second including hysteresis.
   * @param first The fist operand
   * @param second The second operand
   * @param hysteresis The hysteresis factor
   * @param wasSmaller if the first operand was smaller than the second one last time.
   * @return true if the first operand is smaller than the second one after hysteresis was applied.
   */
  template <typename Value, typename Hysteresis>
  constexpr bool smallerThan(const Value& first, const Value& second, const Hysteresis& hysteresis,
                             const bool wasSmaller)
  {
    if (wasSmaller)
    {
      return first < second + hysteresis;
    }
    return first < second - hysteresis;
  }
  /**
   * @brief checks if the first operand is within the boundaries given by the hysteresis
   * @param first The fist operand
   * @param second The second operand
   * @param hysteresis The hysteresis factor
   * @return true if the first operand is equal than the second one after hysteresis was applied.
   */
  template <typename Value, typename Hysteresis>
  constexpr bool equalTo(const Value& first, const Value& second, const Hysteresis& hysteresis)
  {
    return first < second + hysteresis && first > second - hysteresis;
  }

} // namespace Hysteresis
