#pragma once

#include <vector>

#include "Framework/DataType.hpp"

#include "Hardware/Clock.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Line.hpp"

struct LineInfo : public Uni::To, public Uni::From
{
  // a pointer to a line that is stored somewhere else e.g. LineData.lines
  const Line<float>* line;
  // the distance from the robot to the line segment (not infinite long)
  float projectionDistance;
  // the length of the line in meters
  float lineLength;
  // the position of the line in LineData.lines
  size_t lineId;

  LineInfo()
    : line(nullptr)
    , projectionDistance(-1.f)
    , lineLength(-1.f)
    , lineId(-1)
  {
  }

  LineInfo(const Line<float>& line, const float projectionDistance, const float lineLength,
           const int lineId)
    : line(&line)
    , projectionDistance(projectionDistance)
    , lineLength(lineLength)
    , lineId(lineId)
  {
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["projectionDistance"] << projectionDistance;
    value["lineLength"] << lineLength;
    value["lineId"] << lineId;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["projectionDistance"] >> projectionDistance;
    value["lineLength"] >> lineLength;
    value["lineId"] >> lineId;
  }
};

class LineData : public DataType<LineData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"LineData"};
  /// All lines detected by the LineDetection
  std::vector<Line<float>> lines;
  /// All information connected to lines detected by the LineDetection
  std::vector<LineInfo> lineInfos;
  /// Bit-vector with same length as filteredSegments_->vertical storing whether a vertical segment
  /// has been used in the line detection
  std::vector<bool> usedVerticalFilteredSegments;
  /// the timestamp of the image in which they were seen
  Clock::time_point timestamp;
  /// whether the lines are valid
  bool valid = false;
  /**
   * @brief reset sets the lines to a defined state
   */
  void reset() override
  {
    valid = false;
    lines.clear();
    lineInfos.clear();
    usedVerticalFilteredSegments.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["lines"] << lines;
    value["lineInfos"] << lineInfos;
    value["usedVerticalFilteredSegments"] << usedVerticalFilteredSegments;
    value["timestamp"] << timestamp;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["lines"] >> lines;
    value["lineInfos"] >> lineInfos;
    value["usedVerticalFilteredSegments"] >> usedVerticalFilteredSegments;
    value["timestamp"] >> timestamp;
    value["valid"] >> valid;
  }
};
