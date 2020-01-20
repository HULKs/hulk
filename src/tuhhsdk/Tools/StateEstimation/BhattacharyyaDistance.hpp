#pragma once

#include "Tools/Math/Eigen.hpp"

namespace BhattacharyyaDistance
{
  /**
   * @brief compute the Bhattacharyya distance between two normal distributions
   *
   * https://en.wikipedia.org/wiki/Bhattacharyya_distance
   *
   * @param mean1 the mean of the first distribution
   * @param cov1 the cov of the first distribution
   * @param mean2 the mean of the second distribution
   * @param cov2 the cov of the second distribution
   * @tparam n dimension
   * @return the bhattacharyya distance
   */
  template <unsigned int n>
  inline float bhattacharyyaDistance(const Eigen::Matrix<float, n, 1> mean1,
                                     const Eigen::Matrix<float, n, n> cov1,
                                     const Eigen::Matrix<float, n, 1> mean2,
                                     const Eigen::Matrix<float, n, n> cov2)
  {
    const Eigen::Matrix<float, n, n> averageCov = (cov1 + cov2) / 2.f;
    return 0.125f * (mean1 - mean2).transpose() * averageCov.inverse() * (mean1 - mean2) +
           0.5f * std::log(averageCov.determinant() /
                           (std::sqrt(cov1.determinant() * cov2.determinant())));
  }
} // namespace BhattacharyyaDistance
