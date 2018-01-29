#pragma once

#include "Tools/Math/Eigen.hpp"

/// Implementation of a Kalman Filter
/**
 * This class realizes a KalmanFilter
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 */
class KalmanFilter
{
public:
  /** default constructor */
  KalmanFilter();

  /** constructor
   * @param A The state matrix
   * @param b The input Vector
   * @param c The output Vector
   * @param x The initial state Vector
   * @param P Initial Guess on update Matrix
   * @param Q Process noise covariance
   * @param R Measurenment noise covariance
   */
  KalmanFilter(const Matrix2f& A, const Vector2f& b, const Vector2f& c, const Vector2f& x, const Matrix2f& P, const Matrix2f& Q, const float& R);

  /**
   * prediction with measurenment update
   * @param u The input for the state space model
   * @param measure The current measurenment
   * @return The estimated state
   */
  Vector2f predict(const float& u, const float& measure);

  /**
   * set the covariances
   * @param Q process noise covariance
   * @param R measurenment noise covariance
   */
  void setCovariances(const Matrix2f& Q, const float& R);

  /**
   * get Kalman gain
   * @return The Kalman gain Vector
   */
  Vector2f predictGain();


private:
  Vector2f K;
  Matrix2f A;
  Vector2f b;
  Vector2f c;
  Vector2f x;
  Matrix2f P;
  Matrix2f Q;

  float R;
};
