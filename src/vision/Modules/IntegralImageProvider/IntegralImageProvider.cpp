#include <iterator>

#include "IntegralImageProvider.hpp"
#include "print.hpp"

#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image422.hpp"


IntegralImageProvider::IntegralImageProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , imageData_(*this)
  , scale_(*this, "scale", [this] { this->integralImageData_->image.scale = scale_(); })
  , mode_(*this, "mode", [] {})
  , integralImageData_(*this)
{
  const Vector2i size = Image422::get444From422Vector(imageData_->image422.size) / scale_();
  integralImageData_->image = IntegralImage(size, scale_());
}

void IntegralImageProvider::cycle()
{
  Chronometer time(debug(), mount_ + "." + imageData_->identification + "_cycle_time");
  if (!imageData_->is_provided)
  {
    return;
  }
  const Vector2i size = Image422::get444From422Vector(imageData_->image422.size) / scale_();
  integralImageData_->image.resize(size);


  switch (static_cast<Mode>(mode_()))
  {
    case Mode::CB:
      constructIntegralImage<Mode::CB>();
      integralImageData_->valid = true;
      break;
    case Mode::CR:
      constructIntegralImage<Mode::CR>();
      integralImageData_->valid = true;
      break;
    case Mode::GREEN:
      constructIntegralImage<Mode::GREEN>();
      integralImageData_->valid = true;
      break;
    case Mode::GREEN_CHROMATICITY:
      constructIntegralImage<Mode::GREEN_CHROMATICITY>();
      integralImageData_->valid = true;
      break;
    default:
      integralImageData_->valid = false;
  }
}

template <IntegralImageProvider::Mode mode>
inline unsigned int IntegralImageProvider::getValue(const int y, const int x) const
{
  unsigned int value{0};
  const YCbCr422 pixel = imageData_->image422.at(y * scale_(), x * scale_() / 2);
  if constexpr (mode == Mode::CB)
  {
    value = static_cast<unsigned int>(pixel.cb_);
  }
  else if constexpr (mode == Mode::CR)
  {
    value = static_cast<unsigned int>(pixel.cr_);
  }
  else if constexpr (mode == Mode::GREEN)
  {
    const RGBColor rgb = pixel.RGB();
    value = static_cast<unsigned int>(rgb.g);
  }
  else if constexpr (mode == Mode::GREEN_CHROMATICITY)
  {
    const RGBColor rgb = pixel.RGB();
    // invert green chromaticity because the ball has less green chromaticity than the field
    // rescale the value to the same size the other modes provide
    value = static_cast<unsigned int>((1.f - rgb.getChromaticity(rgb.g)) *
                                      static_cast<float>(std::numeric_limits<std::uint8_t>::max()));
  }
  return value;
}


template <IntegralImageProvider::Mode mode>
void IntegralImageProvider::constructIntegralImage()
{
  // initialize first pixel
  integralImageData_->image.at(0, 0) = getValue<mode>(0, 0);

  // initialize first row
  for (int x = 1; x < integralImageData_->image.size.x(); x++)
  {
    unsigned int value = getValue<mode>(0, x);
    integralImageData_->image.at(0, x) = value + integralImageData_->image.at(0, x - 1);
  }

  // initialize first column
  for (int y = 1; y < integralImageData_->image.size.y(); y++)
  {
    unsigned int value = getValue<mode>(y, 0);
    integralImageData_->image.at(y, 0) = value + integralImageData_->image.at(y - 1, 0);
  }

  for (int y = 1; y < integralImageData_->image.size.y(); y++)
  {
    for (int x = 1; x < integralImageData_->image.size.x(); x++)
    {
      unsigned int value = getValue<mode>(y, x);
      value += integralImageData_->image.at(y - 1, x);
      value += integralImageData_->image.at(y, x - 1);
      value -= integralImageData_->image.at(y - 1, x - 1);
      integralImageData_->image.at(y, x) = value;
    }
  }
}
