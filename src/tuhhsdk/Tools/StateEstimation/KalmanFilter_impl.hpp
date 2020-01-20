#pragma once

#include "KalmanFilter.hpp"

template <int n, int m>
KalmanFilter<n, m>::KalmanFilter(const VectorN& x, const MatrixN& P, const MatrixN& F,
                                 const MatrixNM& B, const MatrixN& Q)
  : x_(x)
  , P_(P)
  , F_(F)
  , B_(B)
  , Q_(Q)
{
}

template <int n, int m>
void KalmanFilter<n, m>::predict(const VectorM& u)
{
  // predict next state
  x_ = F_ * x_ + B_ * u;
  P_ = F_ * P_ * F_.transpose() + Q_;
  symmetrifyCovariance(P_);
}

template <int n, int m>
void KalmanFilter<n, m>::predict(const MatrixN& Q, const VectorM& u)
{
  Q_ = Q;
  predict(u);
}

template <int n, int m>
template <int k>
void KalmanFilter<n, m>::update(const Eigen::Matrix<float, k, 1>& z,
                                const Eigen::Matrix<float, k, k>& R,
                                const Eigen::Matrix<float, k, n>& H)
{
  using VectorK = Eigen::Matrix<float, k, 1>;
  using MatrixK = Eigen::Matrix<float, k, k>;
  using MatrixNK = Eigen::Matrix<float, n, k>;

  // compute residual and its covariance
  const VectorK y = z - H * x_;
  const MatrixK S = H * P_ * H.transpose() + R;

  // compute Kalman gain
  const MatrixNK K = P_ * H.transpose() * S.inverse();

  // update
  x_ = x_ + K * y;
  P_ = P_ - K * S * K.transpose();
  symmetrifyCovariance(P_);
}


template <int n, int m>
void KalmanFilter<n, m>::setTransitionMatrix(const MatrixN& F)
{
  F_ = F;
}

template <int n, int m>
typename KalmanFilter<n, m>::VectorN KalmanFilter<n, m>::getState() const
{
  return x_;
}

template <int n, int m>
typename KalmanFilter<n, m>::MatrixN KalmanFilter<n, m>::getCovariance() const
{
  return P_;
}

template <int n, int m>
void KalmanFilter<n, m>::symmetrifyCovariance(KalmanFilter<n, m>::MatrixN& P) const
{
  KalmanFilter<n, m>::MatrixN PTransposed = P.transpose();
  P = 0.5f * (PTransposed + P);
}
