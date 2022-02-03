#pragma once

#include "Tools/Math/Eigen.hpp"

namespace MahalanobisDistance
{
  /**
   * @brief compute the Mahalanobis distance of a point to a distribution with mean and covariance
   *
   * https://en.wikipedia.org/wiki/Mahalanobis_distance
   *
   * @param point the point
   * @param mean the mean of the distribution
   * @param cov the cov of the distribution
   * @tparam n dimension
   * @return the Mahalanobis distance
   */
  template <unsigned int n>
  inline float mahalanobisDistance(const Eigen::Matrix<float, n, 1> point,
                                   const Eigen::Matrix<float, n, 1> mean,
                                   const Eigen::Matrix<float, n, n> cov)
  {
    return std::sqrt((point - mean).transpose() * cov.inverse() * (point - mean));
  }
} // namespace MahalanobisDistance
