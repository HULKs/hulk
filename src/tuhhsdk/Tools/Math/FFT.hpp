#pragma once
//include complex before fftw, so fftw can use it as its complex type.
#include <complex>
#include <fftw3.h>

#include <vector>

typedef std::vector<std::complex<double>> ComplexVector;
typedef std::vector<double> RealVector;

class FFT {
public:
  FFT(unsigned int nfft);
  ~FFT();

  const ComplexVector& fft(const RealVector& input);
  const RealVector& ifft(const ComplexVector& input);
private:
  RealVector realBuffer_;
  ComplexVector complexBuffer_;

  fftw_plan fftPlan_;
  fftw_plan ifftPlan_;
};
