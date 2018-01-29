#pragma once

#include <random>

class Random {
public:
  /**
   * @brief uniformFloat gets a pseudorandom number in the range [min, max)
   * @param min the (inclusive) lowest number that this function may return
   * @param max the (exlusive) highest number that this function may return
   * @return a pseudorandom number in the range [min, max)
   */
  static float uniformFloat(float min = 0, float max = 1);
  /**
   * @brief gaussianFloat gets a number according to a univariate normal distribution
   * @param mean the mean of the distribution
   * @param stddev the standard deviation of the distribution
   * @return a pseudorandom number from a normal distribution
   */
  static float gaussianFloat(float mean, float stddev);
  /**
   * @brief uniformInt gets a pseudorandom number in the range [min, max]
   * @param min the (inclusive) lowest number that this function may return
   * @param max the (inclusive) highest number that this function may return
   * @return a pseudorandom number in the range [min, max]
   */
  static int uniformInt(int min, int max);
private:
  /**
   * @brief Random initializes members
   */
  Random();
  /**
   * @brief getInstance gets an instance of the Random class
   * @param a reference to an instance of the Random class
   */
  static Random& getInstance();
  /// random device, only needed at initialization
  std::random_device rd_;
  /// (pseduo-)random number engine
  std::mt19937 engine_;
};
