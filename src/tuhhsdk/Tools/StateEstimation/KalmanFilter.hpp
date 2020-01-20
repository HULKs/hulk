#pragma once

#include "Tools/Math/Eigen.hpp"

/// Implementation of a Kalman Filter
template <int n, int m>
class KalmanFilter
{
public:
  using VectorN = Eigen::Matrix<float, n, 1>;
  using MatrixN = Eigen::Matrix<float, n, n>;
  using VectorM = Eigen::Matrix<float, m, 1>;
  using MatrixNM = Eigen::Matrix<float, n, m>;

  /**
   * @brief constructor
   * @param x Initial state vector
   * @param P Initial state covariance
   * @param F State transition matrix
   * @param B Control matrix
   * @param Q Process noise covariance
   */
  KalmanFilter(const VectorN& x, const MatrixN& P, const MatrixN& F, const MatrixNM& B,
               const MatrixN& Q);

  /*
   * @brief prediction step
   * @param u control input vector
   */
  void predict(const VectorM& u = VectorM::Zero());

  /*
   * @brief prediction step with specific process covariance
   * @param Q process covariance
   * @param u control input vector
   */
  void predict(const MatrixN& Q, const VectorM& u = VectorM::Zero());

  /*
   * @brief update step with measurement covariance
   * @param z measurement input
   * @param r measurement covariance
   * @param H state to measurement Matrix
   */
  template <int k>
  void update(const Eigen::Matrix<float, k, 1>& z, const Eigen::Matrix<float, k, k>& R,
              const Eigen::Matrix<float, k, n>& H = Eigen::Matrix<float, k, n>::Zero());

  void setTransitionMatrix(const MatrixN& F);

  VectorN getState() const;
  MatrixN getCovariance() const;

private:
  void symmetrifyCovariance(MatrixN& P) const;

  /// current state
  VectorN x_;
  /// state covariance
  MatrixN P_;

  /// state transition matrix
  MatrixN F_;
  /// control matrix
  MatrixNM B_;
  /// process noise covariance
  MatrixN Q_;
};

// Functions with template parameters need to be implemented in the header file.
// https://stackoverflow.com/questions/8752837/undefined-reference-to-template-class-constructor
#include "KalmanFilter_impl.hpp"
