#pragma once

#include <vector>
#include <list>

#include "Framework/DataType.hpp"
#include "Tools/Storage/Image.hpp"

class ColorClass {
public:
  /**
   * @brief Class the actual enumeration of color classes
   */
  enum Class {
    /// no color class or unknown
    NONE = 0,
    /// lines, goal, ball parts, robot parts
    WHITE = 1 << 0,
    /// field
    GREEN = 1 << 1,
    /// old ball (for compatibility reasons)
    RED = 1 << 2,
    /// jerseys
    COLOR = 1 << 3,
    /// robot parts
    GREY = 1 << 4
  };
  /**
   * @brief ColorClass creates an object holding a color class
   * @param c the initial value of the color class
   */
  ColorClass(Class c = Class::NONE) :
    class_(c)
  {
  }
  /**
   * @brief is checks whether this color class contains all the colors that another one contains
   * @param other another color class
   * @return whether this color class contains all the colors that the other one contains
   */
  bool is(const ColorClass other) const
  {
    return (class_ & other.class_) == other.class_;
  }
  /**
   * @brief add adds all color classes from another class to this one
   * @param other another color class
   */
  void add(const ColorClass other)
  {
    class_ = static_cast<Class>(class_ | other.class_);
  }
  /**
   * @brief mask restricts this color class to contain at most the colors that another one contains
   * @param other another color class
   */
  void mask(const ColorClass other)
  {
    class_ = static_cast<Class>(class_ & other.class_);
  }
private:
  /// the actual value of this color class
  Class class_;
};

enum class EdgeType {
  /// an edge that is at one of the image borders
  BORDER,
  /// a rising edge
  RISING,
  /// a falling edge
  FALLING
};

struct Region : public Uni::To, public Uni::From {
  /// upper y coordinate of the region, inclusive
  int start;
  /// lower y coordinate of the region, exclusive (i.e. the first coordinate that does NOT belong to this region)
  int end;
  /// the averaged color of the region
  Color color;
  /// the type of the start region edge
  EdgeType start_edge;
  /// the type of the end region edge
  EdgeType end_edge;
  /// the probability that this region belongs to the field
  float field;

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["start"] << start;
    value["end"] << end;
    value["color_y"] << color.y_;
    value["color_cb"] << color.cb_;
    value["color_cr"] << color.cr_;
    value["start_edge"] << static_cast<int>(start_edge);
    value["end_edge"] << static_cast<int>(end_edge);
    value["field"] << field;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    int valueRead = 0;
    uint32_t unsignedIntRead = 0;
    value["start"] >> start;
    value["end"] >> end;
    value["color_y"] >> unsignedIntRead;
    color.y_ = (uint8_t)unsignedIntRead;
    value["color_cb"] >> unsignedIntRead;
    color.cb_ = (uint8_t)unsignedIntRead;
    value["color_cr"] >> unsignedIntRead;
    color.cr_ = (uint8_t)unsignedIntRead;
    value["start_edge"] >> valueRead;
    start_edge = static_cast<EdgeType>(valueRead);
    value["end_edge"] >> valueRead;
    end_edge = static_cast<EdgeType>(valueRead);
    value["field"] >> field;
  }
};

struct Scanline : public Uni::To, public Uni::From {
  /// an identifier for the scanline, adjacent scanlines have sequential IDs
  int id;
  /// the x coordinate that is common to all regions on this scanline
  int x;
  /// the maximum y coordinate in this scanline
  int y_max;
  /// the regions on this scanline, sorted from top to bottom
  std::vector<Region> regions;

  /**
   * @see function in DataType
   */
  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["id"] << id;
    value["x"] << x;
    value["regions"] << regions;
  }

  /**
   * @see function in DataType
   */
  virtual void fromValue(const Uni::Value& value)
  {
    value["id"] >> id;
    value["x"] >> x;
    value["regions"] >> regions;
  }
};

class ImageRegions : public DataType<ImageRegions> {
public:
  /// vertical scanlines, each of them has vertical regions
  std::vector<Scanline> scanlines;
  /// whether the regions are valid
  bool valid;
  /**
   * @brief reset clears all the vectors
   */
  void reset()
  {
    valid = false;
    scanlines.clear();
  }

  /**
   * @see function in DataType
   */
  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["scanlines"] << scanlines;
    value["valid"] << valid;
  }

  /**
   * @see function in DataType
   */
  virtual void fromValue(const Uni::Value& value)
  {
    value["scanlines"] >> scanlines;
    value["valid"] >> valid;
  }
};
