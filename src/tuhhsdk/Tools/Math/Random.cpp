#include "Random.hpp"

Random::Random() :
  rd_(),
  engine_(rd_())
{
}

Random& Random::getInstance()
{
  static Random instance;
  return instance;
}

float Random::uniformFloat(float min, float max)
{
  std::uniform_real_distribution<float> uniform(min, max);
  return uniform(getInstance().engine_);
}

float Random::gaussianFloat(float mean, float stddev)
{
  std::normal_distribution<float> normal(mean, stddev);
  return normal(getInstance().engine_);
}

int Random::uniformInt(int min, int max)
{
  std::uniform_int_distribution<int> uniform(min, max);
  return uniform(getInstance().engine_);
}
