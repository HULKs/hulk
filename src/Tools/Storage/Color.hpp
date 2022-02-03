#pragma once

#include <algorithm>
#include <cstdint>

struct YCbCr422;
struct RGBColor;

struct Color
{
  constexpr Color() = default;
  /**
   * @brief Color initializes the channels with user-defines values
   * @param y the initial value for the y channel
   * @param cb the initial value for the cb channel
   * @param cr the initial value for the cr channel
   */
  constexpr Color(std::uint8_t y, std::uint8_t cb, std::uint8_t cr)
    : y{y}
    , cb{cb}
    , cr{cr}
  {
  }

  /**
   * @brief Construct a Color (YCbCr) from a YCbCr422 color
   * @param ycbcr422 the color to construct from
   */
  constexpr explicit Color(const YCbCr422& ycbcr422);

  /**
   * @brief Constrcut a Color (YCbCr) from a RGB color
   * @param rgbColor the color in RGB to construct from
   */
  constexpr explicit Color(const RGBColor& rgbColor);

  /// y channel
  std::uint8_t y{};
  /// cb/u channel
  std::uint8_t cb{};
  /// cr/v channel
  std::uint8_t cr{};
  /// static member for red
  static const Color RED;
  /// static member for green
  static const Color GREEN;
  /// static member for blue
  static const Color BLUE;
  /// static member for white
  static const Color WHITE;
  /// static member for black
  static const Color BLACK;
  /// static member for yellow
  static const Color YELLOW;
  /// static member for orange
  static const Color ORANGE;
  /// static member for pink
  static const Color PINK;
  /// static member for transparency
  static const Color TRANSPARENT;

  /**
   * @brief Comparison with another color
   */
  constexpr bool operator==(const Color& other) const
  {
    return y == other.y && cb == other.cb && cr == other.cr;
  }
};

constexpr Color Color::RED{76, 84, 255};
constexpr Color Color::GREEN{149, 43, 21};
constexpr Color Color::BLUE{29, 255, 107};
constexpr Color Color::WHITE{255, 128, 128};
constexpr Color Color::BLACK{0, 128, 128};
constexpr Color Color::YELLOW{208, 16, 146};
constexpr Color Color::ORANGE{151, 42, 201};
constexpr Color Color::PINK{90, 147, 245};
constexpr Color Color::TRANSPARENT{0, 0, 0};

struct RGBColor
{
  /**
   * @brief RGBColor initializes the channels with 0
   */
  constexpr RGBColor() = default;

  /**
   * @brief RGBColor initializes the channels with user-defines values
   * @param R the initial value for the R channel
   * @param G the initial value for the G channel
   * @param B the initial value for the B channel
   */
  constexpr RGBColor(std::uint8_t red, std::uint8_t green, std::uint8_t blue)
    : r{red}
    , g{green}
    , b{blue}
  {
  }

  /**
   * @brief Construct a RGBColor from a YCbCr422 color
   * @param other the YCbCr422 color
   */
  constexpr explicit RGBColor(const YCbCr422& other);

  /// Red channel
  std::uint8_t r{};
  /// Green channel
  std::uint8_t g{};
  /// Blue channel
  std::uint8_t b{};

  /**
   * @brief Comparison with another color
   */
  constexpr bool operator==(const RGBColor& other) const
  {
    return r == other.r && g == other.g && b == other.b;
  }

  /*
   * @brief determines whether the color is saturated in RGB context
   * @return true if the color is saturated
   */
  constexpr bool isSaturated() const
  {
    return r == 255 || g == 255 || b == 255;
  }

  /**
   * @brief get chromaticity of given channel
   * @param value The absolute value of a channel
   * @return the chromaticity of the given value
   */
  constexpr float getChromaticity(const uint8_t value) const
  {
    // avoid division by zero
    if (value > 0)
    {
      return static_cast<float>(value) / static_cast<float>(r + g + b);
    }
    return 0.f;
  }
};

struct YCbCr422
{
  /**
   * @brief YCbCr422 initializes the channels with 0
   */
  constexpr YCbCr422() = default;
  /**
   * @brief YCbCr422 initializes the channels with user-defines values
   * @param y1 the initial value for the y channel
   * @param cb the initial value for the cb channel
   * @param y2 the initial value for the y channel
   * @param cr the initial value for the cr channel
   */
  constexpr YCbCr422(std::uint8_t y1, std::uint8_t cb, std::uint8_t y2, std::uint8_t cr)
    : y1{y1}
    , cb{cb}
    , y2{y2}
    , cr{cr}
  {
  }
  /// y1 channel
  std::uint8_t y1{};
  /// cb/u channel
  std::uint8_t cb{};
  /// y2 channel
  std::uint8_t y2{};
  /// cr/v channel
  std::uint8_t cr{};

  /**
   * @brief Comparison with another color
   */
  constexpr bool operator==(const YCbCr422& other) const
  {
    return y1 == other.y1 && cb == other.cb && y2 == other.y2 && cr == other.cr;
  }

  /**
   * @brief Calculates average over luminance
   */
  constexpr std::uint8_t averagedY() const
  {
    return static_cast<std::uint8_t>(
        (static_cast<std::uint16_t>(y1) + static_cast<std::uint16_t>(y2)) >> 1u);
  }
};

constexpr Color::Color(const YCbCr422& ycbcr422)
  : y{ycbcr422.y1}
  , cb{ycbcr422.cb}
  , cr{ycbcr422.cr}
{
}

constexpr Color::Color(const RGBColor& rgbColor)
  : y{static_cast<uint8_t>(16 + 0.2567890625f * static_cast<float>(rgbColor.r) +
                           0.49631640625f * static_cast<float>(rgbColor.g) +
                           0.09790625f * static_cast<float>(rgbColor.b))}
  , cb{static_cast<uint8_t>(128 - 0.14822265625f * static_cast<float>(rgbColor.r) -
                            0.2909921875f * static_cast<float>(rgbColor.g) +
                            0.43921484375f * static_cast<float>(rgbColor.b))}
  , cr{static_cast<uint8_t>(128 + 0.43921484375f * static_cast<float>(rgbColor.r) -
                            0.3677890625f * static_cast<float>(rgbColor.g) -
                            0.07142578125f * static_cast<float>(rgbColor.b))}
{
}

constexpr RGBColor::RGBColor(const YCbCr422& other)
{
  const float y = other.averagedY();
  const auto centeredCb = static_cast<float>(other.cb - 0x80);
  const auto centeredCr = static_cast<float>(other.cr - 0x80);
  // Conversion from 0-255 ranged YCbCr space to 0-255 ranged rgb color space according to JPEG
  // conversion (https://en.wikipedia.org/wiki/YCbCr#JPEG_conversion)
  r = std::clamp(static_cast<int>(y + 1.40200f * centeredCr), 0, 255);
  g = std::clamp(static_cast<int>(y - 0.34414f * centeredCb - 0.71414f * centeredCr), 0, 255);
  b = std::clamp(static_cast<int>(y + 1.77200f * centeredCb), 0, 255);
}
