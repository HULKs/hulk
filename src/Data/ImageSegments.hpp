#pragma once

#include <list>
#include <utility>
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
  /// first or last edge
  BORDER,
  /// an edge that follows after a robot segment
  START,
  /// an edge that precedes a robot segment
  END,
  /// a rising edge
  RISING,
  /// a falling edge
  FALLING
};

struct Segment : public Uni::To, public Uni::From
{
  Segment() = default;

  Segment(Vector2i start, EdgeType startEdgeType)
    : start(std::move(start))
    , startEdgeType(startEdgeType)
  {
  }

  /// pixel coordinate this segment starts
  Vector2i start;
  /// pixel coordinate this segment ends
  Vector2i end;
  /// median ycbcr422 color
  YCbCr422 ycbcr422;
  /// probability that this region belongs to the field
  float field{0.f};
  /// type of the segment's start edge
  EdgeType startEdgeType{EdgeType::BORDER};
  /// type of the segment's end edge
  EdgeType endEdgeType{EdgeType::BORDER};
  /// number of sampled points within this segment.
  int scanPoints{0};

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["start"] << start;
    value["end"] << end;
    auto pixel = Uni::Value(Uni::ValueType::OBJECT);
    pixel["y1"] << ycbcr422.y1;
    pixel["cb"] << ycbcr422.cb;
    pixel["y2"] << ycbcr422.y2;
    pixel["cr"] << ycbcr422.cr;
    value["ycbcr422"] << pixel;
    value["field"] << field;
    value["startEdgeType"] << static_cast<int>(startEdgeType);
    value["endEdgeType"] << static_cast<int>(endEdgeType);
  }
  void fromValue(const Uni::Value& value) override
  {
    value["start"] >> start;
    value["end"] >> end;
    value["ycbcr422"]["y1"] >> ycbcr422.y1;
    value["ycbcr422"]["cb"] >> ycbcr422.cb;
    value["ycbcr422"]["y2"] >> ycbcr422.y2;
    value["ycbcr422"]["cr"] >> ycbcr422.cr;
    value["field"] >> field;
    int enumValue = 0;
    value["startEdgeType"] >> enumValue;
    startEdgeType = static_cast<EdgeType>(enumValue);
    value["endEdgeType"] >> enumValue;
    endEdgeType = static_cast<EdgeType>(enumValue);
  }
};

struct Scanline : public Uni::To, public Uni::From
{
  Scanline() = default;
  /**
   * @brief Constructs a new Scanline
   * @param scanlineType whether this is a VERTICAL or HORIZONTAL scanline
   */
  explicit Scanline(ScanlineType scanlineType)
    : scanlineType(scanlineType)
  {
  }

  /**
   * @brief Constructs a new Scanline
   * @param scanlineType whether this is a VERTICAL or HORIZONTAL scanline
   * @param id a unique sequential id
   * @param pos the position of the scanline in the image
   * @param maxIndex the maximum coordinate a segment can have in the scanline
   * @param maxElements the maximum number of segments in this scanline, will be reserved
   */
  Scanline(ScanlineType scanlineType, int id, int pos, int maxIndex, int maxElements)
    : scanlineType(scanlineType)
    , id(id)
    , pos(pos)
    , maxIndex(maxIndex)
  {
    segments.reserve(maxElements);
  }

  /// whether this is a vertical or horizontal scanline
  ScanlineType scanlineType = ScanlineType::VERTICAL;
  /// an identifier for the scanline, adjacent scanlines have sequential IDs
  int id = -1;
  /// Its principal position (x coordinate for vertical scanlines and y for horizontal)
  int pos = -1;
  /// maximum position a segment can have (y coordinate for vertical and x for horizontal)
  int maxIndex = -1;
  /// the segments on this scanline
  std::vector<Segment> segments;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["scanlineType"] << static_cast<int>(scanlineType);
    value["id"] << id;
    value["pos"] << pos;
    value["maxIndex"] << maxIndex;
    value["segments"] << segments;
  }
  void fromValue(const Uni::Value& value) override
  {
    int enumValue = 0;
    value["scanlineType"] >> enumValue;
    scanlineType = static_cast<ScanlineType>(enumValue);
    value["id"] >> id;
    value["pos"] >> pos;
    value["maxIndex"] >> maxIndex;
    value["segments"] >> segments;
  }
};

class ImageSegments : public DataType<ImageSegments>
{
public:
  /// the name of this DataType
  DataTypeName name__{"ImageSegments"};
  /// vector of all vertical scanlines containing the segments
  std::vector<Scanline> verticalScanlines;
  /// vector of all horizontal scanlines containing the segments
  std::vector<Scanline> horizontalScanlines;
  /// whether the produced scanlines contain useful data
  bool valid = false;

  /// vector of the scanline's y coordinates for top and bottom camera
  std::array<std::vector<int>, 2> horizontalScanlinePositions;

  /**
   * @brief reset clears all the vectors
   */
  void reset() override
  {
    valid = false;

    for (auto& scanline : verticalScanlines)
    {
      scanline.segments.clear();
      scanline.maxIndex = 0;
    }
    horizontalScanlines.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["verticalScanlines"] << verticalScanlines;
    value["horizontalScanlines"] << horizontalScanlines;
    value["valid"] << valid;
  }
  void fromValue(const Uni::Value& value) override
  {
    value["verticalScanlines"] >> verticalScanlines;
    value["horizontalScanlines"] >> horizontalScanlines;
    value["valid"] >> valid;
  }
};
