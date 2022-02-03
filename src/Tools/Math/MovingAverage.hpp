#pragma once

#include <array>
#include <cstddef>
#include <vector>

/**
 * @brief AbstractMovingAverage abstract class for moving average
 * @tparam T the type of the samples to store
 * @tparam Sum the type of the sum to calculate (may differ from T)
 */
template <typename T, typename Sum>
class AbstractMovingAverage
{
public:
  /**
   * @brief put adds a new sample to the buffer
   * @param sample the sample to add
   */
  virtual void put(T sample) = 0;

  /**
   * @brief getAverage returns the average over the buffer.
   * @return the average value of the last N samples
   */
  virtual Sum getAverage() const = 0;

  /**
   * @brief getSum returns the sum over the buffer
   * @return the sum of the last N samples
   */
  virtual Sum getSum() const = 0;

  /**
   * @brief getRange returns the value range of the buffer
   * @return the range of the last N samples
   */
  virtual Sum getRange() const = 0;
};

/**
 * @brief SimpleArrayMovingAverage calculates the UNWEIGHTED mean of the previous n samples
 *
 * This implementation uses an array to store the samples. The size is required as a template
 * parameter.
 *
 * @tparam T the type of the samples to store
 * @tparam Sum the type of the sum to calculate (may differ from T)
 * @tparam N the amount of samples to keep
 */
template <typename T, typename Sum, std::size_t N>
class SimpleArrayMovingAverage : AbstractMovingAverage<T, Sum>
{
public:
  /**
   * @brief SimpleArrayMovingAverage initializes members
   */
  SimpleArrayMovingAverage()
    : numSamples_(0)
    , sum_(0)
  {
  }

  /**
   * @see AbstractMovingAverage#put
   */
  void put(T sample) override
  {
    if (numSamples_ < N)
    {
      samples_[numSamples_++] = sample;
      sum_ += sample;
    }
    else
    {
      // Subtract the oldest element from the current sum and add the new sample
      T& oldest = samples_[numSamples_++ % N];
      sum_ += sample - oldest;
      oldest = sample;
    }
  }

  /**
   * @see AbstractMovingAverage#getAverage
   */
  Sum getAverage() const override
  {
    return sum_ / std::min(numSamples_, N);
  }

  /**
   * @see AbstractMovingAverage#getSum
   */
  Sum getSum() const override
  {
    return sum_;
  }

  /**
   * @see AbstractMovingAverage#getRange
   */
  Sum getRange() const override
  {
    if (numSamples_ == 0)
    {
      return {};
    }

    Sum lowest = samples_[0];
    Sum highest = samples_[0];
    for (size_t i = 0; i < std::min(numSamples_, N); i++)
    {
      lowest = samples_[i] < lowest ? samples_[i] : lowest;
      highest = samples_[i] > highest ? samples_[i] : highest;
    }

    return highest - lowest;
  }

private:
  /// Ring buffer for all samples
  std::array<T, N> samples_;
  /// The amount of samples currently stored in the buffer. Used during init phase (numSamples < N)
  size_t numSamples_;
  /// The current sum of all elements in the buffer.
  Sum sum_;
};

/**
 * @brief SimpleVectorMovingAverage calculates the UNWEIGHTED mean of the previous n samples
 *
 * This implementation uses a vector to store the samples. The size is required upon construction.
 *
 * @tparam T the type of the samples to store
 * @tparam Sum the type of the sum to calculate (may differ from T)
 *
 */
template <typename T, typename Sum>
class SimpleVectorMovingAverage : AbstractMovingAverage<T, Sum>
{
public:
  /**
   * @brief SimpleVectorMovingAverage initializes members
   * @param n the size of the buffer
   */
  SimpleVectorMovingAverage(const unsigned int n)
    : numSamples_(0)
    , sum_(0)
  {
    samples_.resize(n);
    std::fill(samples_.begin(), samples_.end(), 0);
  }

  /**
   * @see AbstractMovingAverage#put
   */
  void put(T sample) override
  {
    if (numSamples_ < samples_.size())
    {
      samples_[numSamples_++] = sample;
      sum_ += sample;
    }
    else
    {
      // Subtract the oldest element from the current sum and add the new sample. Since vector is
      // specialized for bool references can't be used in this implementation
      // (https://en.wikipedia.org/wiki/Sequence_container_(C%2B%2B)#Specialization_for_bool).
      sum_ += sample - samples_[numSamples_++ % samples_.size()];
      samples_[(numSamples_ - 1) % samples_.size()] = sample;
    }
  }

  /**
   * @see AbstractMovingAverage#getAverage
   */
  Sum getAverage() const override
  {
    return sum_ / std::min(numSamples_, samples_.size());
  }

  /**
   * @see AbstractMovingAverage#getSum
   */
  Sum getSum() const override
  {
    return sum_;
  }

  /**
   * @see AbstractMovingAverage#getRange
   */
  Sum getRange() const override
  {
    if (numSamples_ == 0)
    {
      return {};
    }

    Sum lowest = samples_[0];
    Sum highest = samples_[0];
    for (size_t i = 0; i < numSamples_; i++)
    {
      lowest = samples_[i] < lowest ? samples_[i] : lowest;
      highest = samples_[i] > highest ? samples_[i] : highest;
    }

    return highest - lowest;
  }

private:
  /// Ring buffer for all samples
  std::vector<T> samples_;
  /// The amount of samples currently stored in the buffer. Used during init phase (numSamples < N)
  size_t numSamples_;
  /// The current sum of all elements in the buffer.
  Sum sum_;
};
