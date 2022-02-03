#pragma once

#include "Hardware/Clock.hpp"
#include <algorithm>
#include <array>
#include <functional>
#include <vector>

template <typename Duration, std::size_t N>
class Interpolator
{
public:
  explicit Interpolator(std::function<float(float)> transform = [](float argument) {
    return argument;
  });
  Interpolator(
      const std::array<float, N>& startValue, const std::array<float, N>& endValue,
      Duration duration,
      std::function<float(float)> transform = [](float argument) { return argument; });
  void reset(const std::array<float, N>& startValue, const std::array<float, N>& endValue,
             Duration duration);

  /**
   * @brief step executes one interpolation step
   * @param timeStep the time step for updating the argument
   * @return an array with the interpolated values
   */
  std::array<float, N> step(Duration timeStep);

  /**
   * @brief finished checks if the inteprolation is done
   * @return true iff the interpolation is done completely
   */
  bool isFinished() const;

private:
  std::array<float, N> startValue_{};
  std::array<float, N> endValue_{};
  float argument_{0.f};
  Duration duration_{};
  std::function<float(float)> transform_{[](float argument) { return argument; }};
};

template <typename Duration, std::size_t N>
Interpolator<Duration, N>::Interpolator(std::function<float(float)> transform)
  : transform_{std::move(transform)}
{
}

template <typename Duration, std::size_t N>
Interpolator<Duration, N>::Interpolator(const std::array<float, N>& startValue,
                                        const std::array<float, N>& endValue, Duration duration,
                                        std::function<float(float)> transform)
  : startValue_{startValue}
  , endValue_{endValue}
  , duration_{std::move(duration)}
  , transform_{std::move(transform)}
{
}

template <typename Duration, std::size_t N>
void Interpolator<Duration, N>::reset(const std::array<float, N>& startValue,
                                      const std::array<float, N>& endValue, Duration duration)
{
  startValue_ = startValue;
  endValue_ = endValue;
  duration_ = duration;
  argument_ = 0.f;
}

template <typename Duration, std::size_t N>
std::array<float, N> Interpolator<Duration, N>::step(const Duration timeStep)
{
  // early return for default constructed interpolator (prevent division by zero)
  if (duration_ == Duration{})
  {
    return startValue_;
  }

  // transform into argument domain
  argument_ = std::clamp(argument_ + timeStep / duration_, 0.f, 1.f);
  const auto transformedArgument{transform_(argument_)};

  // assign and return interpolated value
  std::array<float, N> result;
  for (std::size_t i = 0; i < N; i++)
  {
    result[i] = (1 - transformedArgument) * startValue_[i] + transformedArgument * endValue_[i];
  }
  return result;
}

template <typename Duration, std::size_t N>
bool Interpolator<Duration, N>::isFinished() const
{
  return duration_ == Duration{} || argument_ >= 1.f;
}
