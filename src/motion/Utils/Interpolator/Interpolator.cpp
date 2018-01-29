#include <algorithm>

#include "Interpolator.hpp"

Interpolator::Interpolator(const std::vector<float>& start, const std::vector<float>& end, const float time)
{
  reset(start, end, time);
}

void Interpolator::reset(const std::vector<float>& start, const std::vector<float>& end, const float time)
{
  start_= start;
  end_ = end;
  time_ = time;
  t_ = 0;
}

std::vector<float> Interpolator::step(const float dt)
{
  std::vector<float> result(start_.size());
  t_ += dt;
  if (t_ > time_) {
    t_ = time_;
  }
  if (time_ == 0.0f) {
    result = start_;
  } else {
    for (unsigned int i = 0; i < result.size(); i++) {
      result[i] = (1 - t_ / time_) * start_[i] + t_ / time_ * end_[i];
    }
  }
  return result;
}

bool Interpolator::finished()
{
  return t_ >= time_;
}
