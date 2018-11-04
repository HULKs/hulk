#pragma once

#include <cassert>
#include <cmath>
#include <set>
#include <vector>

namespace Statistics
{
  template <typename T>
  inline T mean(const std::vector<T>& vector)
  {
    const unsigned int size = vector.size();
    assert(size > 0 && "size was 0 in mean");
    float mean = 0.f;
    for (unsigned int i = 0; i < size; i++)
    {
      mean += vector[i];
    }
    return mean / size;
  }

  template <typename T>
  inline T standardDeviation(const std::vector<T>& vector, const T mean)
  {
    const unsigned int size = vector.size();
    assert(size > 0 && "size was 0 in standardDeviation");
    float standardDeviation = 0.f;
    for (unsigned int i = 0; i < size; i++)
    {
      standardDeviation += (mean - vector[i]) * (mean - vector[i]);
    }
    return std::sqrt(standardDeviation / size);
  }

  /**
   * @brief median computes the median of five elements
   * http://stackoverflow.com/questions/480960/code-to-calculate-median-of-five-in-c-sharp/2117018#2117018
   */
  template <typename T>
  T median(T a, T b, T c, T d, T e)
  {
    return b < a ? d < c ? b < d ? a < e ? a < d ? e < d ? e : d : c < a ? c : a
                                         : e < d ? a < d ? a : d : c < e ? c : e
                                 : c < e ? b < c ? a < c ? a : c : e < b ? e : b
                                         : b < e ? a < e ? a : e : c < b ? c : b
                         : b < c ? a < e ? a < c ? e < c ? e : c : d < a ? d : a
                                         : e < c ? a < c ? a : c : d < e ? d : e
                                 : d < e ? b < d ? a < d ? a : d : e < b ? e : b : d < b ? d : b
                 : d < c ? a < d ? b < e ? b < d ? e < d ? e : d : c < b ? c : b
                                         : e < d ? b < d ? b : d : c < e ? c : e
                                 : c < e ? a < c ? b < c ? b : c : e < a ? e : a
                                         : a < e ? b < e ? b : e : c < a ? c : a
                         : a < c ? b < e ? b < c ? e < c ? e : c : d < b ? d : b
                                         : e < c ? b < c ? b : c : d < e ? d : e
                                 : d < e ? a < d ? b < d ? b : d : e < a ? e : a
                                         : a < e ? b < e ? b : e : d < a ? d : a;
  }

  /**
   * @brief median computes the median of three elements
   */
  template <typename T>
  T median(T a, T b, T c)
  {
    return a > b ? b > c ? b : a > c ? c : a : a > c ? a : b > c ? c : b;
  }

  /**
   * @brief computes the median of elements within a std::multiset.
   * https://stackoverflow.com/questions/12140635/stdmultiset-and-finding-the-middle-element
   */
  template <class T>
  float median(const std::multiset<T>& data)
  {
    if (data.empty())
    {
      return 0;
    }

    const unsigned int n = data.size();
    float median = 0;

    auto iter = data.cbegin();
    std::advance(iter, n / 2);

    // Middle or average of two middle values
    if (n % 2 == 0)
    {
      const auto iter2 = iter--;
      median = static_cast<float>(*iter + *iter2) / 2; // data[n/2 - 1] AND data[n/2]
    }
    else
    {
      median = *iter;
    }

    return median;
  }
} // namespace Statistics
