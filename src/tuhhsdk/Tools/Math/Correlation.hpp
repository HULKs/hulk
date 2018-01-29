#pragma once

#include <Hardware/AudioInterface.hpp>
#include "FFT.hpp"

class Correlation {
public:
  Correlation(unsigned int size);
  RealVector correlate(RealVector x1, RealVector x2);
private:

  unsigned int size_;
  FFT fft_;
};


