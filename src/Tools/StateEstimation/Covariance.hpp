#pragma once

#include "Tools/Math/Eigen.hpp"

namespace Covariance
{
  /**
   * @brief fixCovariance makes a given covariance symmetric by averaging with
   * its transpose: A = 0.5 (A' * A) @param cov the matrix to be fixed
   */
  inline void fixCovariance(Matrix2f& cov)
  {
    cov(0, 1) = (cov(0, 1) + cov(1, 0)) * .5f;
    cov(1, 0) = cov(0, 1);
  }
  /** @brief fixCovariance makes a given covariance symmetric by averaging with
   * its transpose: A = 0.5 (A' * A) @param cov the matrix to be fixed
   */
  inline void fixCovariance(Matrix3f& cov)
  {
    cov(0, 1) = (cov(0, 1) + cov(1, 0)) * .5f;
    cov(1, 0) = cov(0, 1);

    cov(1, 2) = (cov(1, 2) + cov(2, 1)) * .5f;
    cov(2, 1) = cov(1, 2);

    cov(0, 2) = (cov(0, 2) + cov(2, 0)) * .5f;
    cov(2, 0) = cov(0, 2);
  }
} // namespace Covariance
