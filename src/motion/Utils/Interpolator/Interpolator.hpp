#pragma once

#include <vector>

class Interpolator {
public:
  /**
   * @brief Interpolator initializes members
   * @param start the values at the start
   * @param end the values at the end
   * @param time the duration of the interpolation
   */
  Interpolator(const std::vector<float>& start = std::vector<float>(), const std::vector<float>& end = std::vector<float>(), const float time = 0);
  /**
   * @brief reset resets the interpolation to time 0 with new parameters
   * @param start the values at the start
   * @param end the values at the end
   * @param time the duration of the interpolation
   */
  void reset(const std::vector<float>& start, const std::vector<float>& end, const float time);
  /**
   * @brief step executes one interpolation step
   * @param dt the time step
   * @return a vector with the interpolated values
   */
  std::vector<float> step(const float dt);
  /**
   * @brief finished checks if the inteprolation is done
   * @return true iff the interpolation is done completely
   */
  bool finished();
private:
  /// the values at the start
  std::vector<float> start_;
  /// the values at the end
  std::vector<float> end_;
  /// the current time
  float t_;
  /// the duration of the interpolation
  float time_;
};
