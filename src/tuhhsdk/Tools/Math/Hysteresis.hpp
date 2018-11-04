#pragma once

template <typename T>
class Hysteresis
{
public:
  /**
   * @brief checks if the first operand is greater than the second including hysteresis.
   * @param first The fist operand
   * @param second The second operand
   * @param hysteresis The hysteresis factor
   * @param wasGreater if the first operand was greater than the second one last time.
   * @return true if the first operand is greater than the second one after hysteresis was applied.
   */
  static bool greaterThan(const T& first, const T& second, const T& hysteresis, const bool& wasGreater)
  {
    if (wasGreater)
    {
      return first > second - hysteresis;
    }
    else
    {
      return first > second + hysteresis;
    }
  }
  /**
   * @brief checks if the first operand is smaller than the second including hysteresis.
   * @param first The fist operand
   * @param second The second operand
   * @param hysteresis The hysteresis factor
   * @param wasSmaller if the first operand was smaller than the second one last time.
   * @return true if the first operand is smaller than the second one after hysteresis was applied.
   */
  static bool smallerThan(const T& first, const T& second, const T& hysteresis, const bool& wasSmaller)
  {
    if (wasSmaller)
    {
      return first < second + hysteresis;
    }
    else
    {
      return first < second - hysteresis;
    }
  }
private:
};
