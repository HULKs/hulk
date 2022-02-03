#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Math/Circle.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/Math/Rectangle.hpp"

/**
 * @brief Represents point data labeled with annotate.
 * NOTE: these points are in normalized coordinates in range [0,1].
 * @author Georg Felbinger
 */
class LabelPoint : public Uni::From, public Uni::To
{
public:
  /// the normalized x coordinates
  float x;
  /// the normalized y coordinates
  float y;

  void fromValue(const Uni::Value& value) override
  {
    value["x"] >> x;
    value["y"] >> y;
  }
  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["x"] << x;
    value["y"] << y;
  }
};

/**
 * @brief Represents a labeled bounding box on an image from annotate.
 * @author Georg Felbinger
 */
class LabelBox : public Uni::From, public Uni::To
{
public:
  /// the label describing this box, e.g. 'robot', 'ball' or 'penaltyspot'
  std::string label;
  /// the topLeft point of the bounding box in normalized image coordinates
  LabelPoint start;
  /// the normalized size of the bounding box
  LabelPoint size;
  /// the rectangle described by this box in image coordinates
  Rectangle<int> box;

  void fromValue(const Uni::Value& value) override
  {
    value["label"] >> label;
    value["start"] >> start;
    value["size"] >> size;
  }
  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["label"] << label;
    value["start"] << start;
    value["size"] << size;
  }
};

class LabelLine : public Uni::From, public Uni::To
{
public:
  /// the label describing this line, e.g. 'line' or 'goalPost'
  std::string label;
  /// the first line point in normalized image coordinates
  LabelPoint start;
  /// the second line point in normalized image coordinates
  LabelPoint end;
  /// the line described by this LabelLine in image coordinates
  Line<int> line;

  void fromValue(const Uni::Value& value) override
  {
    value["label"] >> label;
    value["start"] >> start;
    value["end"] >> end;
  }
  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["label"] << label;
    value["start"] << start;
    value["end"] << end;
  }
};

/**
 * @brief Represents the data stored alongside with images by annotate.
 * @author Georg Felbinger
 */
class LabelData : public DataType<LabelData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"LabelData"};
  /// the absolute path to the image
  std::string image;
  /// the label describing the situation on the image, e.g. 'game', 'unclear' or 'other'
  std::string label;
  /// bounding box labels on the image.
  std::vector<LabelBox> boxes;
  /// line labels on the image
  std::vector<LabelLine> lines;

  void fromValue(const Uni::Value& value) override
  {
    value["label"] >> label;
    value["boxes"] >> boxes;
    value["lines"] >> lines;
  }
  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["label"] << label;
    value["boxes"] << boxes;
    value["lines"] << lines;
  }
  void reset() override
  {
    label = "";
    boxes.clear();
    lines.clear();
  }
};
