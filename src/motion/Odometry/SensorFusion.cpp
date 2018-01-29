#include "SensorFusion.hpp"
#include "Modules/NaoProvider.h"

SensorFusion::SensorFusion(const ModuleBase& module)
  : initialized_(false)
  , reset_(module, "reset",
           [=] {
             setOrientation({0, 0, 0});
           })
  , accelweight_(module, "accelweight", [] {})
  , sensor_update_rate_(module, "sensor_update_rate", [] {})
  , gravity_(module, "gravity", [] {})
  , gyro_bias_alpha_(module, "gyro_bias_alpha", [] {}) /// TODO: Maybe do some sort of auto calibration
  , acceleration_threshold_(module, "acceleration_threshold", [] {})
  , delta_angular_velocity_threshold_(module, "delta_angular_velocity_threshold", [] {})
  , angular_velocity_threshold_(module, "angular_velocity_threshold", [] {})
  , gyro_prev_(0, 0, 0)
  , gyro_bias_(0, 0, 0)
  , global_to_local_(1, 0, 0, 0)
{
}

void SensorFusion::update(const Vector3f& extGyro, const Vector3f& extAccel)
{
  Vector3d eigenExtGyro(extGyro.x(), extGyro.y(), -extGyro.z());
  Vector3d eigenExtAccel(-extAccel.x(), +extAccel.y(), -extAccel.z());

  if (!initialized_ && eigenExtAccel.norm() >= 1.0)
  {
    calculateOrientation(eigenExtGyro, eigenExtAccel);
    initialized_ = true;
    return;
  }
  else if (!initialized_ && eigenExtAccel.norm() < 1.0)
  {
    // Calculating the orientation of the nao while falling (low gravity)
    // would lead to big errors anyway
    return;
  }

  updateGyroBias(eigenExtGyro, eigenExtAccel);
  updateOrientationGyro(eigenExtGyro);
  updateOrientationAccel(eigenExtAccel);
}

bool SensorFusion::checkSteadyState(const Vector3d& extGyro, const Vector3d& extAccel)
{
  double acc_norm = extAccel.norm();
  if (std::fabs(acc_norm - gravity_()) > acceleration_threshold_())
  {
    return false;
  }

  Vector3d gyro_delta = (extGyro - gyro_prev_).cwiseAbs();
  if (gyro_delta(0) > delta_angular_velocity_threshold_() || gyro_delta(1) > delta_angular_velocity_threshold_() ||
      gyro_delta(2) > delta_angular_velocity_threshold_())
  {
    return false;
  }

  Vector3d gyro = (extGyro - gyro_bias_).cwiseAbs();
  if (gyro(0) > angular_velocity_threshold_() || gyro(1) > angular_velocity_threshold_() || gyro(2) > angular_velocity_threshold_())
  {
    return false;
  }

  return true;
}

void SensorFusion::updateGyroBias(const Vector3d& extGyro, const Vector3d& extAccel)
{
  bool steady_state = checkSteadyState(extGyro, extAccel);

  if (steady_state)
  {
    gyro_bias_ = gyro_bias_alpha_() * (extGyro - gyro_bias_);
  }

  gyro_prev_ = extGyro;
}

void SensorFusion::calculateOrientation(const Vector3d& /*extGyro*/, const Vector3d& extAccel)
{
  Vector3d accel = extAccel.normalized();
  double q0, q1, q2, q3;

  if (accel(2) >= 0)
  {
    q0 = std::sqrt((accel(2) + 1.0) * 0.5);
    q1 = -accel(1) / (2 * q0);
    q2 = +accel(0) / (2 * q0);
    q3 = 0;
  }
  else
  {
    double intermediate = std::sqrt((1 - accel(2)) * 0.5);
    q0 = -accel(1) / (2.0 * intermediate);
    q1 = intermediate;
    q2 = 0;
    q3 = +accel(0) / (2.0 * intermediate);
  }

  global_to_local_ = Quaterniond(q0, q1, q2, q3);
}

void SensorFusion::updateOrientationGyro(const Vector3d& extGyro)
{
  // See paper page 15
  Vector3d gyro = extGyro - gyro_bias_;

  Quaterniond omega(0, gyro(0), gyro(1), gyro(2));
  Quaterniond dq = omega * global_to_local_;

  global_to_local_.coeffs() += dq.coeffs() * -0.5 / sensor_update_rate_();

  // Last thing to do is normalize the quaternion
  global_to_local_.normalize();
}

void SensorFusion::updateOrientationAccel(const Vector3d& extAccel)
{
  double eps = 0.9; /// SLERP threshold
  double alpha = 0;
  double n = extAccel.norm();
  double error = std::abs((n - gravity_())) / gravity_();

  if (error <= 0.1)
  {
    alpha = 1;
  }
  else if (error <= 0.2)
  {
    alpha = -10 * (error - 0.2);
  }

  if (alpha == 0)
  {
    return;
  }

  alpha *= accelweight_();

  // Normalize factors, we need the original vector later for the adaptive gain.
  Vector3d a = extAccel;
  a /= n;
  Vector3d gv = global_to_local_.inverse()._transformVector(a);

  double gx = gv(0);
  double gy = gv(1);
  double gz = gv(2);

  // Calculate the correction quaternion
  double q0 = std::sqrt((gz + 1.0) * 0.5);
  double q1 = -gy / (2.0 * q0);
  double q2 = +gx / (2.0 * q0);
  Quaterniond dqa(q0, q1, q2, 0);

  Quaterniond eye = Quaterniond::Identity();
  Quaterniond dqab;

  // Do interpolation between current frame and accelerometer frame
  // Based on how close we are to the "correct" frame
  // If we are far away use LERP
  // Otherwise use SLERP
  double dot = eye.dot(dqa);
  if (dot > eps)
  {
    dqab = (1 - alpha) * eye.coeffs() + alpha * dqa.coeffs();
  }
  else
  {
    dqab = eye.slerp(alpha, dqa);
  }

  dqab.normalize();
  global_to_local_ *= dqab;
  global_to_local_.normalize();
}

void SensorFusion::setOrientation(const Vector3d& orient)
{
  reset_() = false;
  // Code from: https://en.wikipedia.org/wiki/Conversion_between_quaternions_and_Euler_angles

  auto& roll = orient(0);
  auto& pitch = orient(1);
  auto& yaw = orient(2);

  Quaterniond q;
  double t0 = std::cos(yaw * 0.5f);
  double t1 = std::sin(yaw * 0.5f);
  double t2 = std::cos(roll * 0.5f);
  double t3 = std::sin(roll * 0.5f);
  double t4 = std::cos(pitch * 0.5f);
  double t5 = std::sin(pitch * 0.5f);

  q.w() = t0 * t2 * t4 + t1 * t3 * t5;
  q.x() = t0 * t3 * t4 - t1 * t2 * t5;
  q.y() = t0 * t2 * t5 + t1 * t3 * t4;
  q.z() = t1 * t2 * t4 - t0 * t3 * t5;
  initialized_ = true;
  global_to_local_ = q.inverse();
}

Vector3f SensorFusion::getOrientation() const
{
  // Code from: https://en.wikipedia.org/wiki/Conversion_between_quaternions_and_Euler_angles
  auto q = global_to_local_.inverse();
  double ysqr = q.y() * q.y();

  // roll (x-axis rotation)
  double t0 = +2.0f * (q.w() * q.x() + q.y() * q.z());
  double t1 = +1.0f - 2.0f * (q.x() * q.x() + ysqr);
  double roll = std::atan2(t0, t1);

  // pitch (y-axis rotation)
  double t2 = +2.0f * (q.w() * q.y() - q.z() * q.x());
  t2 = t2 > 1.0f ? 1.0f : t2;
  t2 = t2 < -1.0f ? -1.0f : t2;
  double pitch = std::asin(t2);

  // yaw (z-axis rotation)
  double t3 = +2.0f * (q.w() * q.z() + q.x() * q.y());
  double t4 = +1.0f - 2.0f * (ysqr + q.z() * q.z());
  double yaw = std::atan2(t3, t4);

  return Vector3f(roll, pitch, yaw);
}

Matrix3f SensorFusion::getBodyTilt() const
{
  Vector3f rpy = getOrientation();
  // the tilt should not contain the yaw. Thus one obtains from rotation matrix:
  // R(yaw, pitch, roll) = R(yaw) * R(pitch) * R(roll)
  //
  // (cos(rpy.z)*cos(rpy.y), cos(rpy.z)*sin(rpy.y)*sin(rpy.x)-sin(rpy.z)*cos(rpy.x), cos(rpy.z)*sin(rpy.y)*cos(rpy.x)+sin(rpy.z)*sin(rpy.x),
  //  sin(rpy.z)*cos(rpy.y), sin(rpy.z)*sin(rpy.y)*sin(rpy.x)+cos(rpy.z)*cos(rpy.x), sin(rpy.z)*sin(rpy.y)*cos(rpy.x)-cos(rpy.z)*sin(rpy.x),
  //  -sin(rpy.y),           cos(rpy.y)*sin(rpy.x),                                  cos(rpy.y)*cos(rpy.x));
  //
  // By canceling some ones and zeros:
  const float sx = sin(rpy.x());
  const float cx = cos(rpy.x());
  const float sy = sin(rpy.y());
  const float cy = cos(rpy.y());

  Matrix3f ret;
  ret << cy, sy * sx, sy * cx, 0, cx, -sx, -sy, cy * sx, cy * cx;
  return ret;
}

Vector3f SensorFusion::getAxisAngles() const
{
  auto q = global_to_local_.inverse();
  float theta = 2 * std::acos(q.x());

  Vector3f w;
  if (theta != 0)
  {
    float s = 1 / (std::sin(theta / 2));
    w.x() = static_cast<float>(s * q.y());
    w.y() = static_cast<float>(s * q.z());
    w.z() = static_cast<float>(s * q.w());
  }

  return w * theta;
}
