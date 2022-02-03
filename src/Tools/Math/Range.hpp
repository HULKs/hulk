#pragma once

#include <algorithm>

#include "Tools/Storage/UniValue/UniValue.h"

template <typename T>
class Range : public Uni::From, public Uni::To
{
public:
  /**
   * @brief Range constructs a range given its minimum and maximum
   * @param min the minimum value of the range (inclusive)
   * @param max the maximum value of the range (inclusive)
   */
  Range(const T& min = T(), const T& max = T())
    : min(min)
    , max(max)
  {
    assert(max >= min);
  }

  /**
   * @brief clipToGivenRange calculates the clipped value from a given val to the interval/range
   * from min to max. (not the min max of the instance)
   * @param val the value to calculate the clipped value from (const, NOT modified!!)
   * @param min the minimum of the range
   * @param max the maximum of the range
   * @return the clipped value
   */
  static T clipToGivenRange(const T& val, const T& min, const T& max)
  {
    assert(max >= min);
    return val < min ? min : val > max ? max : val;
  }

  /**
   * @brief clipToZeroOne clipping to a range from 0 to 1;
   * @param val the value used vor clipping
   * @return the clipped value
   */
  static T clipToZeroOne(const T& val)
  {
    return clipToGivenRange(val, 0.f, 1.f);
  }

  /**
   * @brief clip Calculates the clipped T of given val to the range of this instance (using own min
   * max)
   * @param val the vlaue to calculate the clipped value from
   * @return the clipped value
   */
  T clip(const T& val) const
  {
    assert(max >= min);
    return clipToGivenRange(val, min, max);
  }

  /**
   * @brief intersect sets this interval to the intersection with another interval
   * If the intersection is empty, the minimum and maximum are set to the value that is closes to
   * the other range.
   * @param min2 the minimum value of the other range (inclusive)
   * @param max2 the maximum value of the other range (inclusive)
   */
  void intersect(const T& min2, const T& max2)
  {
    assert(max2 >= min2);

    if (max2 <= min)
    {
      max = min;
      return;
    }
    else if (min2 >= max)
    {
      min = max;
      return;
    }
    else
    {
      min = std::max(min, min2);
      max = std::min(max, max2);
    }
    assert(max >= min);
  }

  /**
   * @brief intersect sets this interval to the intersection with another interval
   * If the intersection is empty, the minimum and maximum are set to the value that is closes to
   * the other range.
   * @param other the other range that this range is intersected with
   *
   */
  void intersect(const Range<T>& other)
  {
    intersect(other.min, other.max);
  }

  /**
   * @brief fromValue converts a Uni::Value to this
   * @param value the value that should be converted to this class
   */
  void fromValue(const Uni::Value& value)
  {
    assert(value.type() == Uni::ValueType::ARRAY);
    assert(value.size() == 2);
    value.at(0) >> min;
    value.at(1) >> max;
    assert(max >= min);
  }

  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the value that this class should be converted to
   */
  void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::ARRAY);
    value.at(0) << min;
    value.at(1) << max;
  }

  /// the minimum value of the range (inclusive)
  T min;
  /// the maximum value of the range (inclusive)
  T max;
};
