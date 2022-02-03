#include "Tools/Math/Correlation.hpp"
#include "Framework/Log/Log.hpp"
#include <algorithm>

Correlation::Correlation(unsigned int size)
  : size_(size)
  , fft_(size)
{
}

RealVector Correlation::correlate(RealVector x1, RealVector x2)
{
  if (x1.size() > size_)
  {
    throw std::invalid_argument("x1 must be of size smaller or equal to Correlation::size_");
  }

  if (x2.size() > size_)
  {
    throw std::invalid_argument("x2 must be of size smaller or equal to Correlation::size_");
  }

  // zero padding;
  while (x1.size() < size_)
  {
    x1.push_back(0);
  }

  while (x2.size() < size_)
  {
    x2.push_back(0);
  }

  std::reverse(x2.begin(), x2.end());

  ComplexVector X1 = fft_.fft(x1);
  ComplexVector X2 = fft_.fft(x2);

  ComplexVector correlation(size_);
  for (unsigned int k = 0; k < size_; ++k)
  {
    // use the analytic signal by applying hilbert transform:
    double h = 0;
    if (k < size_ / 2)
    {
      h = -1;
    }
    else if (k > size_ / 2)
    {
      h = 1;
    }
    //    double h = 1;
    correlation[k] = (X1[k] * X2[k]) * h;
  }

  std::stringstream ss;
  RealVector corr = fft_.ifft(correlation);
  for (double& k : corr)
  {
    k = fabs(k / size_);
  }

  return corr;
}
