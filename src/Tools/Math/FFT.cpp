#include "Tools/Math/FFT.hpp"
#include "Framework/Log/Log.hpp"

FFT::FFT(unsigned int nfft)
{
  realBuffer_.resize(nfft);
  complexBuffer_.resize(nfft);

  // fft
  fftPlan_ =
      fftw_plan_dft_r2c_1d(static_cast<int>(nfft), realBuffer_.data(),
                           // NOLINTNEXTLINE(hicpp-signed-bitwise)
                           reinterpret_cast<fftw_complex*>(complexBuffer_.data()), FFTW_ESTIMATE);
  // ifft
  ifftPlan_ = fftw_plan_dft_c2r_1d(static_cast<int>(nfft),
                                   reinterpret_cast<fftw_complex*>(complexBuffer_.data()),
                                   // NOLINTNEXTLINE(hicpp-signed-bitwise)
                                   realBuffer_.data(), FFTW_ESTIMATE);
}

FFT::~FFT()
{
  fftw_destroy_plan(fftPlan_);
  fftw_destroy_plan(ifftPlan_);
}

const ComplexVector& FFT::fft(const RealVector& input)
{
  if (input.size() != realBuffer_.size())
  {
    Log<M_TUHHSDK>(LogLevel::ERROR)
        << "input.size = " << static_cast<int>(input.size())
        << ", realBuffer.size() = " << static_cast<int>(realBuffer_.size());
    throw std::runtime_error("FFT: Input size does not match allocated buffer size.");
  }

  std::copy(input.begin(), input.end(), realBuffer_.begin());
  fftw_execute(fftPlan_);
  return complexBuffer_;
}

const RealVector& FFT::ifft(const ComplexVector& input)
{
  if (input.size() != complexBuffer_.size())
  {
    Log<M_TUHHSDK>(LogLevel::ERROR)
        << "input.size = " << static_cast<int>(input.size())
        << ", complexBuffer.size() = " << static_cast<int>(realBuffer_.size());
    throw std::runtime_error("IFFT: Input size does not match allocated buffer size.");
  }

  std::copy(input.begin(), input.end(), complexBuffer_.begin());
  fftw_execute(ifftPlan_);
  return realBuffer_;
}
