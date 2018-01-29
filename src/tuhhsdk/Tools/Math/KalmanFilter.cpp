#include "KalmanFilter.h"

using namespace std;

KalmanFilter::KalmanFilter()
  : K(0, 0)
  , A(Matrix2f::Zero())
  , b(1, 0)
  , c(1, 0)
  , x(0, 0)
  , P(Matrix2f::Zero())
  , Q(Matrix2f::Zero())
  , R(1)
{
}


KalmanFilter::KalmanFilter(const Matrix2f& A, const Vector2f& b, const Vector2f& c, const Vector2f& x, const Matrix2f& P, const Matrix2f& Q, const float& R)
  : K()
  , A(A)
  , b(b)
  , c(c)
  , x(x)
  , P(P)
  , Q(Q)
  , R(R)
{
  // Do nothing else.
}


void KalmanFilter::setCovariances(const Matrix2f& Q, const float& R)
{
  this->Q = Q;
  this->R = R;
}

Vector2f KalmanFilter::predict(const float& u, const float& measure)
{
  // predict next state
  x = A * x + b * u;

  // calculate update matrix
  P = A * P * A.transpose() + Q;

  // calculate Kalman gain
  K = P * c / (c.transpose() * P * c + R);

  // correct state
  x = x + K * (measure - c.dot(x));

  // update of P
  P = P - (Matrix2f() << K * c.x(), K * c.y()).finished() * P;

  return x;
}

Vector2f KalmanFilter::predictGain()
{
  // calculate update matrix
  P = A * P * A.transpose() + Q;

  // calculate Kalman gain
  K = P * c / (c.transpose() * P * c + R);

  // update of P
  P = P - (Matrix2f() << K * c.x(), K * c.y()).finished() * P;

  return K;
}
