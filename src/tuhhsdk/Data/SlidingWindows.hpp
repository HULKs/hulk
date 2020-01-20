#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Rectangle.hpp"

/**
 * @brief Represents a single sliding window.
 * @author Georg Felbinger
 */
struct SlidingWindow : public Uni::From, public Uni::To
{
  /// The rectangle enclosing this window
  Rectangle<int> window;
  /// Edge points given by the horizontal image segmentation
  std::vector<Vector2i> edgePoints;
  /// The amount of scanlines with fieldcolor within this window
  float fieldColor;
  /// The amount of scanlines within this window
  int scanlines;

  SlidingWindow()
    : window({0, 0}, {0, 0})
    , edgePoints()
    , fieldColor(0.f)
    , scanlines(0)
  {
  }

  SlidingWindow(const Vector2i& tl, const Vector2i& br)
    : window(tl, br)
    , edgePoints()
    , fieldColor(0.f)
    , scanlines(0)
  {
  }

  SlidingWindow(const SlidingWindow& o)
    : window(o.window)
    , edgePoints(o.edgePoints)
    , fieldColor(o.fieldColor)
    , scanlines(0)
  {
  }

  /**
   * @brief reset resets the members
   */
  void reset()
  {
    edgePoints.clear();
    fieldColor = 0.f;
    scanlines = 0;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["window"] << window;
    value["edgePoints"] << edgePoints;
    value["fieldColor"] << fieldColor;
    value["scanlines"] << scanlines;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["window"] >> window;
    value["edgePoints"] >> edgePoints;
    value["fieldColor"] >> fieldColor;
    value["scanlines"] >> scanlines;
  }

  /**
   * @brief Caculates the center of mass of the enclosed edgePoints
   */
  inline Vector2i calculateCom() const
  {
    Vector2i com = Vector2i::Zero();
    for (const auto& p : edgePoints)
    {
      com += p;
    }
    com /= edgePoints.size();
    return com;
  }
};

/**
 * @brief Represents a row of sliding windows
 * @author Georg Felbinger
 */
struct SlidingWindowRow : public Uni::From, public Uni::To
{
  /// the upper y pixel coordinate of this row
  int top;
  /// the lower y pixel coordinate of this row
  int bottom;
  /// the contained SlidingWindows
  std::vector<SlidingWindow> windows;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["top"] << top;
    value["bottom"] << bottom;
    value["windows"] << windows;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["top"] >> top;
    value["bottom"] >> bottom;
    value["windows"] >> windows;
  }
};

/**
 * @brief Represents a image discretized by pseudo-projected sliding windows
 * @author Georg Felbinger
 */
class SlidingWindows : public DataType<SlidingWindows>
{
public:
  /// the name of this DataType
  DataTypeName name = "SlidingWindows";
  /// The contained rows of sliding windows
  std::array<std::vector<SlidingWindowRow>, 2> rows;
  /// Whether the content of this datatype was valid this cycle
  bool valid = false;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["rows"] << rows;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["rows"] >> rows;
    value["valid"] >> valid;
  }

  void reset() override
  {
    valid = false;
  }
};
