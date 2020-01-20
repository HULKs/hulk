#pragma once

#include <list>
#include <vector>

#include "Framework/DataType.hpp"
#include "Tools/Storage/Image422.hpp"

enum class ScanlineType
{
  VERTICAL,
  HORIZONTAL
};

enum class EdgeType
{
  // last edge (no following edges)
  BORDER,
  // an edge that follows after a robot segment
  START,
  // an edge that precedes a robot segment
  END,
  // a rising edge
  RISING,
  // a falling edge
  FALLING
};

struct Scanline;
struct Segment : public Uni::To, public Uni::From
{
  Segment() = default;

  Segment(Vector2i start, EdgeType startEdgeType)
    : start(start)
    , startEdgeType(startEdgeType)
  {
  }

  Vector2i start;
  Vector2i end;
  // the median ycbcr422 color
  YCbCr422 ycbcr422;
  // the probability that this region belongs to the field
  float field;
  // the type of the segment start edge
  EdgeType startEdgeType;
  // the type of the segment end edge
  EdgeType endEdgeType;
  // The number of sampled points within this segment.
  int scanPoints;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["start"] << start;
    value["end"] << end;
    value["ycbcr422_y1"] << ycbcr422.y1_;
    value["ycbcr422_y2"] << ycbcr422.y2_;
    value["ycbcr422_cb"] << ycbcr422.cb_;
    value["ycbcr422_cr"] << ycbcr422.cr_;
    value["field"] << field;
    value["startEdgeType"] << static_cast<int>(startEdgeType);
    value["endEdgeType"] << static_cast<int>(endEdgeType);
  }

  void fromValue(const Uni::Value& value) override
  {
    int valueRead = 0;
    uint32_t unsignedIntRead = 0;
    value["start"] >> start;
    value["end"] >> end;
    value["ycbcr422_y1"] >> unsignedIntRead;
    ycbcr422.y1_ = (uint8_t)unsignedIntRead;
    value["ycbcr422_y2"] >> unsignedIntRead;
    ycbcr422.y2_ = (uint8_t)unsignedIntRead;
    value["ycbcr422_cb"] >> unsignedIntRead;
    ycbcr422.cb_ = (uint8_t)unsignedIntRead;
    value["ycbcr422_cr"] >> unsignedIntRead;
    ycbcr422.cr_ = (uint8_t)unsignedIntRead;
    value["field"] >> field;
    value["startEdgeType"] >> valueRead;
    startEdgeType = static_cast<EdgeType>(valueRead);
    value["endEdgeType"] >> valueRead;
    endEdgeType = static_cast<EdgeType>(valueRead);
  }
};

struct Scanline : public Uni::To, public Uni::From
{
  Scanline(ScanlineType scanlineType)
    : scanlineType(scanlineType)
  {
  }

  // TODO Preallocate the horizontal scanlines and set the id
  // an identifier for the scanline, adjacent scanlines have sequential IDs
  int id = -1;
  // Its principal position (x coordinate for vertical scanlines and y for horizontal scanlines)
  int pos = -1;
  ScanlineType scanlineType;
  // the segments on this scanline
  std::vector<Segment> segments;

  /**
   * @see function in DataType
   */
  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["id"] << id;
    value["segments"] << segments;
    value["scanlineType"] << static_cast<int>(scanlineType);
  }

  /**
   * @see function in DataType
   */
  virtual void fromValue(const Uni::Value& /*value*/) {}
};

struct VerticalScanline : public Scanline
{
  VerticalScanline()
    : Scanline(ScanlineType::VERTICAL)
  {
  }
  // the maximum y coordinate in this scanline to allow cutting out robot parts
  int yMax = -1;

  /**
   * @see function in DataType
   */
  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    Scanline::toValue(value);
    value["yMax"] << yMax;
  }

  /**
   * @see function in DataType
   */
  virtual void fromValue(const Uni::Value& /*value*/) {}
};

struct HorizontalScanline : public Scanline
{
  HorizontalScanline()
    : Scanline(ScanlineType::HORIZONTAL)
  {
  }
  Vector2i step;
  /**
   * @see function in DataType
   */
  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    Scanline::toValue(value);
    value["step"] << step;
  }

  /**
   * @see function in DataType
   */
  virtual void fromValue(const Uni::Value& /*value*/) {}
};

class ImageSegments : public DataType<ImageSegments>
{
public:
  /// the name of this DataType
  DataTypeName name = "ImageSegments";
  std::vector<VerticalScanline> verticalScanlines;
  std::vector<HorizontalScanline> horizontalScanlines;
  // TODO: Think about the single valid boolean. Probably split it up for vertical and horizontal
  // scanlines.
  bool valid = false;
  // whether the regions were reinitialized
  bool reinitialized = false;
  // the amount of vertical scanlines
  int numVerticalScanlines;
  // size of the current image
  Vector2i imageSize{0, 0};
  /**
   * @brief Lookup table for (horizontal) scanline distances.
   *
   * Contains a step of approx. 3cm on the worlds ground for every camera and image row, given the
   * {@link cam2Ground_}.
   */
  std::array<std::vector<Vector2i>, 2> scanGrids;

  /**
   * @brief initializes the scanlines for a certain image size and number of scanlines
   * @param imageSize size of the used image
   * @param numVerticalScanlines the amount of vertical scanlines used to downsample the image
   */
  void init(const Vector2i& imageSize, int numVerticalScanlines)
  {
    reset();
    this->imageSize = imageSize;
    this->numVerticalScanlines = numVerticalScanlines;

    verticalScanlines.clear();
    verticalScanlines.resize(numVerticalScanlines);
    const int scanlineSpacing = imageSize.x() / numVerticalScanlines;

    for (int i = 0; i < numVerticalScanlines; ++i)
    {
      verticalScanlines[i].pos = (scanlineSpacing / 2) + scanlineSpacing * i;
      verticalScanlines[i].id = i + 1;
      verticalScanlines[i].yMax = imageSize.y() - 1;
      verticalScanlines[i].segments.reserve(imageSize.y());
    }
    reinitialized = true;
  }

  // TODO: Prealocation (init) for horizontal scanlines

  /**
   * @brief reset clears all the vectors
   */
  void reset()
  {
    valid = false;
    reinitialized = false;

    for (auto& scanline : verticalScanlines)
    {
      scanline.segments.clear();
      scanline.yMax = imageSize.y() - 1;
    }
    horizontalScanlines.clear();
  }

  /**
   * @see function in DataType
   */
  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["verticalScanlines"] << verticalScanlines;
    value["horizontalScanlines"] << horizontalScanlines;
    value["valid"] << valid;
  }

  /**
   * @see function in DataType
   */
  virtual void fromValue(const Uni::Value& value)
  {
    value["verticalScanlines"] >> verticalScanlines;
    value["horizontalScanlines"] >> horizontalScanlines;
    value["valid"] >> valid;
  }
};
