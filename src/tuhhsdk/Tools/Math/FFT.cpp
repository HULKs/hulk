#include "FFT.hpp"
#include <print.h>

FFT::FFT(unsigned int nfft) :
  realBuffer_(),
  complexBuffer_()
{
  realBuffer_.resize(nfft);
  complexBuffer_.resize(nfft);

  //fft
  fftPlan_ = fftw_plan_dft_r2c_1d(nfft, realBuffer_.data(), reinterpret_cast<fftw_complex*>(complexBuffer_.data()), FFTW_ESTIMATE);
  //ifft
  ifftPlan_ = fftw_plan_dft_c2r_1d(nfft, reinterpret_cast<fftw_complex*>(complexBuffer_.data()), realBuffer_.data(), FFTW_ESTIMATE);
}

FFT::~FFT()
{
  fftw_destroy_plan(fftPlan_);
  fftw_destroy_plan(ifftPlan_);
}

const ComplexVector& FFT::fft(const RealVector& input)
{
  if (input.size() != realBuffer_.size()) {
    Log(LogLevel::ERROR) << "input.size = " << (int)input.size() << ", realBuffer.size() = " << (int)realBuffer_.size();
    throw std::runtime_error("FFT: Input size does not match allocated buffer size.");
  }

  std::copy(input.begin(), input.end(), realBuffer_.begin());
  fftw_execute(fftPlan_);
  return complexBuffer_;
}

const RealVector& FFT::ifft(const ComplexVector& input)
{
  if (input.size() != complexBuffer_.size()) {
    Log(LogLevel::ERROR) << "input.size = " << (int)input.size() << ", complexBuffer.size() = " << (int)realBuffer_.size();
    throw std::runtime_error("IFFT: Input size does not match allocated buffer size.");
  }

  std::copy(input.begin(), input.end(), complexBuffer_.begin());
  fftw_execute(ifftPlan_);
  return realBuffer_;
}
